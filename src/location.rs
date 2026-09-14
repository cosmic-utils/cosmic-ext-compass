// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::{Subscription, stream};
use futures::{SinkExt, StreamExt};
use std::{any::TypeId, time::Duration};
use zbus::zvariant::OwnedObjectPath;

const SERVICE: &str = "org.freedesktop.GeoClue2";
const MANAGER_PATH: &str = "/org/freedesktop/GeoClue2/Manager";
const DESKTOP_ID: &str = "org.cosmic_utils.compass";
const EXACT_ACCURACY: u32 = 8;
const RETRY_INITIAL: Duration = Duration::from_secs(1);
const RETRY_MAX: Duration = Duration::from_secs(30);

#[zbus::proxy(
    interface = "org.freedesktop.GeoClue2.Manager",
    default_service = "org.freedesktop.GeoClue2",
    default_path = "/org/freedesktop/GeoClue2/Manager"
)]
trait GeoClueManager {
    fn get_client(&self) -> zbus::Result<OwnedObjectPath>;
    fn delete_client(&self, client: OwnedObjectPath) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.GeoClue2.Client",
    default_service = "org.freedesktop.GeoClue2"
)]
trait GeoClueClient {
    fn start(&self) -> zbus::Result<()>;
    fn stop(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn location(&self) -> zbus::Result<OwnedObjectPath>;

    #[zbus(property)]
    fn set_desktop_id(&self, desktop_id: &str) -> zbus::Result<()>;

    #[zbus(property)]
    fn set_distance_threshold(&self, meters: u32) -> zbus::Result<()>;

    #[zbus(property)]
    fn set_time_threshold(&self, seconds: u32) -> zbus::Result<()>;

    #[zbus(property)]
    fn set_requested_accuracy_level(&self, level: u32) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.GeoClue2.Location",
    default_service = "org.freedesktop.GeoClue2"
)]
trait GeoClueLocation {
    #[zbus(property)]
    fn latitude(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn longitude(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn accuracy(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn altitude(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn description(&self) -> zbus::Result<String>;
}

#[derive(Clone, Debug)]
pub enum LocationEvent {
    Fix(LocationFix),
    Unavailable,
    AccessDenied,
    Failed(String),
}

pub fn subscription() -> Subscription<LocationEvent> {
    struct LocationSubscription;
    Subscription::run_with(TypeId::of::<LocationSubscription>(), |_| {
        stream::channel(8, async |mut output| {
            let mut retry_delay = RETRY_INITIAL;
            loop {
                let error = match monitor(&mut output).await {
                    Ok(()) => "GeoClue location stream ended".to_owned(),
                    Err(error) => error,
                };
                tracing::warn!(
                    %error,
                    service = SERVICE,
                    path = MANAGER_PATH,
                    "location service failed; retrying"
                );
                let event = if error.contains("AccessDenied")
                    || error.contains("NotAuthorized")
                    || error.contains("PermissionDenied")
                {
                    LocationEvent::AccessDenied
                } else {
                    LocationEvent::Failed(error)
                };
                if output.send(event).await.is_err() {
                    break;
                }
                tokio::time::sleep(retry_delay).await;
                retry_delay = retry_delay.saturating_mul(2).min(RETRY_MAX);
            }
        })
    })
}

async fn monitor(output: &mut futures::channel::mpsc::Sender<LocationEvent>) -> Result<(), String> {
    let connection = zbus::Connection::system()
        .await
        .map_err(|error| error.to_string())?;
    let manager = GeoClueManagerProxy::new(&connection)
        .await
        .map_err(|error| error.to_string())?;
    let client_path = manager
        .get_client()
        .await
        .map_err(|error| error.to_string())?;
    let client = GeoClueClientProxy::builder(&connection)
        .path(client_path.clone())
        .map_err(|error| error.to_string())?
        .build()
        .await
        .map_err(|error| error.to_string())?;

    client
        .set_desktop_id(DESKTOP_ID)
        .await
        .map_err(|error| error.to_string())?;
    client
        .set_requested_accuracy_level(EXACT_ACCURACY)
        .await
        .map_err(|error| error.to_string())?;
    client
        .set_distance_threshold(1)
        .await
        .map_err(|error| error.to_string())?;
    client
        .set_time_threshold(1)
        .await
        .map_err(|error| error.to_string())?;

    // Subscribe before starting so the first fix cannot race setup.
    let mut locations = client.receive_location_changed().await;
    client.start().await.map_err(|error| error.to_string())?;

    while let Some(changed) = locations.next().await {
        match changed.get().await {
            Ok(path) if path.as_str() != "/" => {
                emit_location(&connection, path, output).await;
            }
            Ok(_) => {
                if output.send(LocationEvent::Unavailable).await.is_err() {
                    break;
                }
            }
            Err(error) => {
                let _ = client.stop().await;
                let _ = manager.delete_client(client_path).await;
                return Err(error.to_string());
            }
        }
    }

    let stop_result = client.stop().await.map_err(|error| error.to_string());
    let delete_result = manager
        .delete_client(client_path)
        .await
        .map_err(|error| error.to_string());
    stop_result.and(delete_result)
}

async fn emit_location(
    connection: &zbus::Connection,
    path: OwnedObjectPath,
    output: &mut futures::channel::mpsc::Sender<LocationEvent>,
) {
    let proxy = match GeoClueLocationProxy::builder(connection).path(path) {
        Ok(builder) => match builder.build().await {
            Ok(proxy) => proxy,
            Err(error) => {
                let _ = output.send(LocationEvent::Failed(error.to_string())).await;
                return;
            }
        },
        Err(error) => {
            let _ = output.send(LocationEvent::Failed(error.to_string())).await;
            return;
        }
    };

    let values = futures::join!(
        proxy.latitude(),
        proxy.longitude(),
        proxy.accuracy(),
        proxy.altitude(),
        proxy.description(),
    );
    let values = (
        values.0.map_err(|error| error.to_string()),
        values.1.map_err(|error| error.to_string()),
        values.2.map_err(|error| error.to_string()),
        values.3.map_err(|error| error.to_string()),
        values.4.map_err(|error| error.to_string()),
    );
    let event = location_event_from_properties(values);
    let _ = output.send(event).await;
}

type LocationProperties = (
    Result<f64, String>,
    Result<f64, String>,
    Result<f64, String>,
    Result<f64, String>,
    Result<String, String>,
);

fn location_event_from_properties(values: LocationProperties) -> LocationEvent {
    let (latitude, longitude, accuracy, altitude, description) = values;
    let required = latitude.and_then(|latitude| {
        longitude.and_then(|longitude| accuracy.map(|accuracy| (latitude, longitude, accuracy)))
    });
    match required {
        Ok((latitude, longitude, accuracy)) => LocationFix::try_new(
            latitude,
            longitude,
            accuracy,
            altitude.unwrap_or(f64::MIN),
            &description.unwrap_or_default(),
        )
        .map_or(LocationEvent::Unavailable, LocationEvent::Fix),
        Err(error) => LocationEvent::Failed(format!("failed to read GeoClue location: {error}")),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocationFix {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: f64,
    pub altitude_m: Option<f64>,
    pub place_name: Option<String>,
}

impl LocationFix {
    #[must_use]
    pub fn try_new(
        latitude: f64,
        longitude: f64,
        accuracy_m: f64,
        altitude_m: f64,
        description: &str,
    ) -> Option<Self> {
        if !latitude.is_finite()
            || !longitude.is_finite()
            || !accuracy_m.is_finite()
            || !(-90.0..=90.0).contains(&latitude)
            || !(-180.0..=180.0).contains(&longitude)
            || accuracy_m < 0.0
        {
            return None;
        }

        let altitude_m = (altitude_m.is_finite() && altitude_m != f64::MIN).then_some(altitude_m);
        let place_name = meaningful_place_name(description);

        Some(Self {
            latitude,
            longitude,
            accuracy_m,
            altitude_m,
            place_name,
        })
    }
}

/// Stable demonstration fix at Burg Eltz, matching the public reference
/// coordinates and elevation used by the demo and its regressions.
#[must_use]
pub fn demo_location() -> LocationFix {
    LocationFix {
        latitude: 50.205_055_4,
        longitude: 7.336_597_1,
        accuracy_m: 12.0,
        altitude_m: Some(320.0),
        place_name: Some("Burg Eltz, Rhineland-Palatinate".to_owned()),
    }
}

#[must_use]
pub fn format_coordinates(latitude: f64, longitude: f64) -> String {
    let (latitude, latitude_hemisphere) = dms(latitude, "N", "S");
    let (longitude, longitude_hemisphere) = dms(longitude, "E", "W");
    format!("{latitude} {latitude_hemisphere} {longitude} {longitude_hemisphere}")
}

fn dms(value: f64, positive: &'static str, negative: &'static str) -> (String, &'static str) {
    let hemisphere = if value.is_sign_negative() {
        negative
    } else {
        positive
    };
    let total_seconds = (value.abs() * 3_600.0).round() as u64;
    let degrees = total_seconds / 3_600;
    let minutes = total_seconds % 3_600 / 60;
    let seconds = total_seconds % 60;
    (format!("{degrees}°{minutes}′{seconds}″"), hemisphere)
}

fn meaningful_place_name(description: &str) -> Option<String> {
    let description = description.trim();
    let normalized = description.to_ascii_lowercase();
    let generic = description.is_empty()
        || matches!(
            normalized.as_str(),
            "gps" | "wifi" | "network" | "modem gps"
        )
        || normalized.starts_with("geoip")
        || normalized.contains("fallback");
    (!generic).then(|| description.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{LocationEvent, location_event_from_properties};

    #[test]
    fn optional_property_failures_preserve_a_valid_location_fix() {
        let event = location_event_from_properties((
            Ok(50.205_055_4),
            Ok(7.336_597_1),
            Ok(12.0),
            Err("altitude unavailable".to_owned()),
            Err("description unavailable".to_owned()),
        ));

        let LocationEvent::Fix(fix) = event else {
            panic!("valid coordinates and accuracy should produce a fix");
        };
        assert_eq!(fix.altitude_m, None);
        assert_eq!(fix.place_name, None);
    }
}
