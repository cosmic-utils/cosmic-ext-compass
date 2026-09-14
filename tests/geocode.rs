// SPDX-License-Identifier: GPL-3.0-only

use compass::geocode::{
    CacheKey, CachedLookup, GeocodeErrorKind, LookupSource, ReverseGeocodeCache, locale_preference,
    lookup_place, place_name_from_json, request_delay,
};
use i18n_embed::unic_langid::LanguageIdentifier;
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn temp_cache_path() -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "compass-geocode-cache-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ))
        .join("reverse-geocode.json")
}

#[test]
fn locale_preference_uses_the_requested_language_order() {
    let german: LanguageIdentifier = "de-DE".parse().unwrap();
    let english: LanguageIdentifier = "en".parse().unwrap();

    assert_eq!(locale_preference(&[german, english]), "de-DE,en");
    assert_eq!(locale_preference(&[]), "en");
}

#[test]
fn cache_key_changes_with_displayed_arcsecond_or_locale() {
    let base = CacheKey::for_display(50.205_055_4, 7.336_597_1, "en-US,en");
    let same_display = CacheKey::for_display(50.205_065_4, 7.336_607_1, "en-US,en");
    let moved_arcsecond =
        CacheKey::for_display(50.205_055_4 + 1.0 / 3_600.0, 7.336_597_1, "en-US,en");
    let german = CacheKey::for_display(50.205_055_4, 7.336_597_1, "de-DE,de");

    assert_eq!(base, same_display);
    assert_ne!(base, moved_arcsecond);
    assert_ne!(base, german);
}

#[test]
fn persistent_cache_invalidates_on_key_change() {
    let path = temp_cache_path();
    let cache = ReverseGeocodeCache::at(path.clone());
    let english = CacheKey::for_display(50.205_055_4, 7.336_597_1, "en-US,en");
    let german = CacheKey::for_display(50.205_055_4, 7.336_597_1, "de-DE,de");

    cache
        .store(&english, Some("Wierschem, Rhineland-Palatinate"))
        .unwrap();
    assert_eq!(
        cache.lookup(&english).unwrap(),
        CachedLookup::Hit(Some("Wierschem, Rhineland-Palatinate".to_owned()))
    );
    assert_eq!(cache.lookup(&german).unwrap(), CachedLookup::Miss);

    let _ = std::fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_file(parent.join("reverse-geocode.lock"));
        let _ = std::fs::remove_dir(parent);
    }
}

#[test]
fn persistent_cache_keeps_multiple_coordinate_and_locale_entries() {
    let path = temp_cache_path();
    let cache = ReverseGeocodeCache::at(path.clone());
    let english = CacheKey::for_display(50.205_055_4, 7.336_597_1, "en");
    let german = CacheKey::for_display(50.205_055_4, 7.336_597_1, "de");

    cache.store(&english, Some("Wierschem")).unwrap();
    cache
        .store(&german, Some("Wierschem, Rheinland-Pfalz"))
        .unwrap();

    assert_eq!(
        cache.lookup(&english).unwrap(),
        CachedLookup::Hit(Some("Wierschem".to_owned()))
    );
    assert_eq!(
        cache.lookup(&german).unwrap(),
        CachedLookup::Hit(Some("Wierschem, Rheinland-Pfalz".to_owned()))
    );

    let _ = std::fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_file(parent.join("reverse-geocode.lock"));
        let _ = std::fs::remove_dir(parent);
    }
}

#[test]
fn localized_place_name_prefers_settlement_and_state() {
    let json = r#"{
        "display_name": "Burg Eltz, Wierschem, Rheinland-Pfalz, Deutschland",
        "address": {
            "village": "Wierschem",
            "state": "Rheinland-Pfalz",
            "country": "Deutschland"
        }
    }"#;

    assert_eq!(
        place_name_from_json(json).unwrap().as_deref(),
        Some("Wierschem, Rheinland-Pfalz")
    );
}

#[test]
fn nominatim_requests_are_limited_to_one_per_minute() {
    assert_eq!(request_delay(1_000, 1_250), Duration::from_millis(59_750));
    assert_eq!(request_delay(1_000, 61_000), Duration::ZERO);
    assert_eq!(request_delay(0, 500), Duration::ZERO);
}

#[test]
fn nominatim_http_errors_are_classified() {
    assert_eq!(
        GeocodeErrorKind::for_status(404),
        GeocodeErrorKind::NoResult
    );
    assert_eq!(
        GeocodeErrorKind::for_status(403),
        GeocodeErrorKind::Forbidden
    );
    assert_eq!(
        GeocodeErrorKind::for_status(429),
        GeocodeErrorKind::RateLimited
    );
    assert_eq!(
        GeocodeErrorKind::for_status(503),
        GeocodeErrorKind::ServiceUnavailable
    );
    assert_eq!(GeocodeErrorKind::for_status(400), GeocodeErrorKind::Other);
}

#[tokio::test]
#[ignore = "requires the public Nominatim service"]
async fn live_nominatim_resolves_the_public_burg_eltz_demo_location() {
    let path = temp_cache_path();
    let cache = ReverseGeocodeCache::at(path.clone());
    let key = CacheKey::for_display(50.205_055_4, 7.336_597_1, "en");

    let first = lookup_place(&cache, &key, "https://nominatim.openstreetmap.org/")
        .await
        .expect("public Burg Eltz lookup should succeed");
    assert!(
        first
            .place_name
            .as_deref()
            .is_some_and(|name| !name.trim().is_empty())
    );
    assert_eq!(first.source, LookupSource::Network);

    let second = lookup_place(&cache, &key, "https://nominatim.openstreetmap.org/")
        .await
        .expect("cached Burg Eltz lookup should succeed");
    assert_eq!(second.source, LookupSource::Cache);
    assert_eq!(second.place_name, first.place_name);

    let _ = std::fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_file(parent.join("reverse-geocode.lock"));
        let _ = std::fs::remove_dir(parent);
    }
}

#[tokio::test]
async fn nominatim_uses_locale_identification_and_persistent_cache() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let length = socket.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..length]).into_owned();
        let body = r#"{"display_name":"Burg Eltz","address":{"village":"Wierschem","state":"Rheinland-Pfalz"}}"#;
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
        request
    });

    let path = temp_cache_path();
    let cache = ReverseGeocodeCache::at(path.clone());
    let key = CacheKey::for_display(50.205_055_4, 7.336_597_1, "de-DE,de");
    let endpoint = format!("http://{address}/");
    let first = lookup_place(&cache, &key, &endpoint).await.unwrap();
    let second = lookup_place(&cache, &key, &endpoint).await.unwrap();
    let request = server.join().unwrap();

    assert_eq!(
        first.place_name.as_deref(),
        Some("Wierschem, Rheinland-Pfalz")
    );
    assert_eq!(first.source, LookupSource::Network);
    assert_eq!(second.source, LookupSource::Cache);
    assert!(request.starts_with("GET /reverse?"));
    assert!(request.contains("accept-language=de-DE%2Cde"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("user-agent: cosmic-compass/")
    );

    let _ = std::fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_file(parent.join("reverse-geocode.lock"));
        let _ = std::fs::remove_dir(parent);
    }
}

#[tokio::test]
async fn nominatim_rejects_oversized_response_bodies() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let length = socket.read(&mut request).unwrap();
        assert!(length > 0);
        let mut body =
            r#"{"display_name":"Wierschem","address":{"village":"Wierschem"}}"#.to_owned();
        body.push_str(&" ".repeat(70 * 1024));
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });

    let path = temp_cache_path();
    let cache = ReverseGeocodeCache::at(path.clone());
    let key = CacheKey::for_display(50.205_055_4, 7.336_597_1, "en");
    let endpoint = format!("http://{address}/");

    assert_eq!(
        lookup_place(&cache, &key, &endpoint).await,
        Err(GeocodeErrorKind::InvalidResponse)
    );
    server.join().unwrap();

    let _ = std::fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_file(parent.join("reverse-geocode.lock"));
        let _ = std::fs::remove_dir(parent);
    }
}
