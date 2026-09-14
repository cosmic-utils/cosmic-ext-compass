// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::{Subscription, stream};
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);
const REQUEST_INTERVAL_MS: u128 = 60_000;
const MAX_CACHE_ENTRIES: usize = 512;
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[must_use]
pub fn locale_preference(languages: &[i18n_embed::unic_langid::LanguageIdentifier]) -> String {
    if languages.is_empty() {
        "en".to_owned()
    } else {
        languages
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct CacheKey {
    latitude_arcseconds: i32,
    longitude_arcseconds: i32,
    locale: String,
}

impl CacheKey {
    #[must_use]
    pub fn for_display(latitude: f64, longitude: f64, locale: &str) -> Self {
        Self {
            latitude_arcseconds: (latitude * 3_600.0).round() as i32,
            longitude_arcseconds: (longitude * 3_600.0).round() as i32,
            locale: locale.to_owned(),
        }
    }

    fn latitude(&self) -> f64 {
        f64::from(self.latitude_arcseconds) / 3_600.0
    }

    fn longitude(&self) -> f64 {
        f64::from(self.longitude_arcseconds) / 3_600.0
    }

    fn locale(&self) -> &str {
        &self.locale
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeocodeErrorKind {
    NoResult,
    Forbidden,
    RateLimited,
    ServiceUnavailable,
    Network,
    InvalidResponse,
    Other,
}

impl GeocodeErrorKind {
    #[must_use]
    pub fn for_status(status: u16) -> Self {
        match status {
            404 => Self::NoResult,
            403 => Self::Forbidden,
            429 => Self::RateLimited,
            500..=599 => Self::ServiceUnavailable,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LookupSource {
    Cache,
    Network,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupResult {
    pub place_name: Option<String>,
    pub source: LookupSource,
}

#[derive(Clone, Debug)]
pub enum GeocodeEvent {
    Resolved {
        key: CacheKey,
        place_name: Option<String>,
    },
    Failed {
        key: CacheKey,
        kind: GeocodeErrorKind,
    },
}

pub fn subscription(
    fix: &crate::location::LocationFix,
    locale: String,
) -> Subscription<GeocodeEvent> {
    let key = CacheKey::for_display(fix.latitude, fix.longitude, &locale);
    let identity = (std::any::TypeId::of::<ReverseGeocodeSubscription>(), key);
    Subscription::run_with(identity, reverse_geocode_stream)
}

fn reverse_geocode_stream(
    data: &(std::any::TypeId, CacheKey),
) -> impl futures::Stream<Item = GeocodeEvent> + use<> {
    let key = data.1.clone();
    stream::channel(1, async move |mut output| {
        let cache_root = std::env::var_os("XDG_CACHE_HOME")
            .filter(|path| Path::new(path).is_absolute())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")));
        let event = if let Some(cache_root) = cache_root {
            let cache = ReverseGeocodeCache::at(
                cache_root
                    .join("org.cosmic_utils.compass")
                    .join("reverse-geocode.json"),
            );
            let endpoint = std::env::var("COMPASS_NOMINATIM_URL")
                .unwrap_or_else(|_| "https://nominatim.openstreetmap.org/".to_owned());
            match lookup_place(&cache, &key, &endpoint).await {
                Ok(result) => GeocodeEvent::Resolved {
                    key: key.clone(),
                    place_name: result.place_name,
                },
                Err(kind) => GeocodeEvent::Failed {
                    key: key.clone(),
                    kind,
                },
            }
        } else {
            GeocodeEvent::Failed {
                key: key.clone(),
                kind: GeocodeErrorKind::Other,
            }
        };
        let _ = output.send(event).await;
        std::future::pending::<()>().await;
    })
}

struct ReverseGeocodeSubscription;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CachedLookup {
    Miss,
    Hit(Option<String>),
}

#[derive(Clone, Debug)]
pub struct ReverseGeocodeCache {
    path: PathBuf,
}

#[derive(Deserialize, Serialize)]
struct CacheDocument {
    version: u8,
    entries: Vec<CacheEntry>,
}

#[derive(Deserialize, Serialize)]
struct CacheEntry {
    key: CacheKey,
    place_name: Option<String>,
}

impl ReverseGeocodeCache {
    #[must_use]
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    fn acquire_lock(&self) -> io::Result<fs::File> {
        use std::os::unix::fs::OpenOptionsExt;

        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        set_private_directory_permissions(parent)?;
        let lock_path = parent.join("reverse-geocode.lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(lock_path)?;
        lock.lock()?;
        Ok(lock)
    }

    pub fn lookup(&self, key: &CacheKey) -> io::Result<CachedLookup> {
        let _lock = self.acquire_lock()?;
        self.lookup_unlocked(key)
    }

    fn lookup_unlocked(&self, key: &CacheKey) -> io::Result<CachedLookup> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(CachedLookup::Miss),
            Err(error) => return Err(error),
        };
        let document: CacheDocument = serde_json::from_str(&contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if document.version != 2 {
            return Ok(CachedLookup::Miss);
        }
        Ok(document
            .entries
            .into_iter()
            .find(|entry| entry.key == *key)
            .map_or(CachedLookup::Miss, |entry| {
                CachedLookup::Hit(entry.place_name)
            }))
    }

    pub fn store(&self, key: &CacheKey, place_name: Option<&str>) -> io::Result<()> {
        let _lock = self.acquire_lock()?;
        self.store_unlocked(key, place_name)
    }

    fn store_unlocked(&self, key: &CacheKey, place_name: Option<&str>) -> io::Result<()> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        set_private_directory_permissions(parent)?;

        let mut entries = match fs::read_to_string(&self.path) {
            Ok(contents) => serde_json::from_str::<CacheDocument>(&contents)
                .ok()
                .filter(|document| document.version == 2)
                .map(|document| document.entries)
                .unwrap_or_default(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error),
        };
        entries.retain(|entry| entry.key != *key);
        entries.push(CacheEntry {
            key: key.clone(),
            place_name: place_name.map(str::to_owned),
        });
        if entries.len() > MAX_CACHE_ENTRIES {
            entries.drain(..entries.len() - MAX_CACHE_ENTRIES);
        }
        let document = CacheDocument {
            version: 2,
            entries,
        };
        let contents = serde_json::to_vec(&document)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let (temp_path, mut temp) = create_private_temp_file(parent, &self.path)?;
        let write_result = (|| {
            temp.write_all(&contents)?;
            temp.sync_all()?;
            fs::rename(&temp_path, &self.path)
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        write_result
    }
}

#[must_use]
pub fn request_delay(last_request_ms: u128, now_ms: u128) -> std::time::Duration {
    if last_request_ms == 0 {
        return std::time::Duration::ZERO;
    }
    let elapsed = now_ms.saturating_sub(last_request_ms);
    std::time::Duration::from_millis(
        u64::try_from(REQUEST_INTERVAL_MS.saturating_sub(elapsed)).unwrap_or(0),
    )
}

fn unix_time_millis() -> Result<u128, GeocodeErrorKind> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|_| GeocodeErrorKind::Other)
}

fn read_last_request(lock: &mut fs::File) -> io::Result<u128> {
    lock.rewind()?;
    let mut value = String::new();
    lock.read_to_string(&mut value)?;
    Ok(value.trim().parse().unwrap_or(0))
}

fn record_request(lock: &mut fs::File, timestamp_ms: u128) -> io::Result<()> {
    lock.set_len(0)?;
    lock.rewind()?;
    write!(lock, "{timestamp_ms}")?;
    lock.sync_data()
}

pub async fn lookup_place(
    cache: &ReverseGeocodeCache,
    key: &CacheKey,
    endpoint: &str,
) -> Result<LookupResult, GeocodeErrorKind> {
    let lock_cache = cache.clone();
    let mut cache_lock = tokio::task::spawn_blocking(move || lock_cache.acquire_lock())
        .await
        .map_err(|_| GeocodeErrorKind::Other)?
        .map_err(|_| GeocodeErrorKind::Other)?;

    match cache.lookup_unlocked(key) {
        Ok(CachedLookup::Hit(place_name)) => {
            return Ok(LookupResult {
                place_name,
                source: LookupSource::Cache,
            });
        }
        Ok(CachedLookup::Miss) => {}
        Err(error) => tracing::warn!(%error, "could not read reverse-geocode cache"),
    }

    let now_ms = unix_time_millis()?;
    let last_request_ms =
        read_last_request(&mut cache_lock).map_err(|_| GeocodeErrorKind::Other)?;
    let delay = request_delay(last_request_ms, now_ms);
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }
    record_request(&mut cache_lock, unix_time_millis()?).map_err(|_| GeocodeErrorKind::Other)?;

    let url = reqwest::Url::parse(endpoint)
        .and_then(|url| url.join("reverse"))
        .map_err(|_| GeocodeErrorKind::Other)?;
    let client = reqwest::Client::builder()
        .user_agent(format!(
            "COSMIC-Compass/{} (https://cosmic-utils.org)",
            env!("GIT_VERSION")
        ))
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|_| GeocodeErrorKind::Network)?;
    let mut response = client
        .get(url)
        .query(&[
            ("format", "jsonv2".to_owned()),
            ("addressdetails", "1".to_owned()),
            ("zoom", "18".to_owned()),
            ("lat", key.latitude().to_string()),
            ("lon", key.longitude().to_string()),
            ("accept-language", key.locale().to_owned()),
        ])
        .send()
        .await
        .map_err(|_| GeocodeErrorKind::Network)?;

    if !response.status().is_success() {
        return Err(GeocodeErrorKind::for_status(response.status().as_u16()));
    }

    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(GeocodeErrorKind::InvalidResponse);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| GeocodeErrorKind::Network)?
    {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(GeocodeErrorKind::InvalidResponse);
        }
        body.extend_from_slice(&chunk);
    }
    let body = String::from_utf8(body).map_err(|_| GeocodeErrorKind::InvalidResponse)?;
    let place_name = place_name_from_json(&body).map_err(|_| GeocodeErrorKind::InvalidResponse)?;
    if let Err(error) = cache.store_unlocked(key, place_name.as_deref()) {
        tracing::warn!(%error, "could not update reverse-geocode cache");
    }

    Ok(LookupResult {
        place_name,
        source: LookupSource::Network,
    })
}

fn create_private_temp_file(parent: &Path, destination: &Path) -> io::Result<(PathBuf, fs::File)> {
    use std::os::unix::fs::OpenOptionsExt;

    let stem = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("reverse-geocode.json");
    for _ in 0..64 {
        let id = TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".{stem}.tmp-{}-{id}", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique cache temporary file",
    ))
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[derive(Deserialize)]
struct NominatimResponse {
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    address: NominatimAddress,
}

#[derive(Default, Deserialize)]
struct NominatimAddress {
    city: Option<String>,
    town: Option<String>,
    village: Option<String>,
    municipality: Option<String>,
    hamlet: Option<String>,
    county: Option<String>,
    state: Option<String>,
    country: Option<String>,
}

pub fn place_name_from_json(json: &str) -> Result<Option<String>, serde_json::Error> {
    let response: NominatimResponse = serde_json::from_str(json)?;
    let settlement = response
        .address
        .city
        .or(response.address.town)
        .or(response.address.village)
        .or(response.address.municipality)
        .or(response.address.hamlet)
        .or(response.address.county);
    let region = response.address.state.or(response.address.country);
    let name = match (settlement, region) {
        (Some(settlement), Some(region)) if settlement != region => {
            Some(format!("{settlement}, {region}"))
        }
        (Some(settlement), _) => Some(settlement),
        (None, Some(region)) => Some(region),
        (None, None) => (!response.display_name.trim().is_empty()).then_some(response.display_name),
    };
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::ReverseGeocodeCache;
    use std::{sync::mpsc, time::Duration};

    #[test]
    fn cache_lock_has_one_stable_identity() {
        let directory =
            std::env::temp_dir().join(format!("compass-geocode-lock-test-{}", std::process::id()));
        let cache = ReverseGeocodeCache::at(directory.join("reverse-geocode.json"));
        let first_lock = cache.acquire_lock().unwrap();
        let contender_cache = ReverseGeocodeCache::at(directory.join("reverse-geocode.json"));
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let contender = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let _lock = contender_cache.acquire_lock().unwrap();
            acquired_tx.send(()).unwrap();
        });

        started_rx.recv().unwrap();
        assert!(acquired_rx.recv_timeout(Duration::from_millis(50)).is_err());
        drop(first_lock);
        acquired_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        contender.join().unwrap();

        let _ = std::fs::remove_file(directory.join("reverse-geocode.lock"));
        let _ = std::fs::remove_dir(directory);
    }
}
