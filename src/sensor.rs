// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::{Subscription, stream};
use futures::{SinkExt, StreamExt, select};
use std::{any::TypeId, time::Duration};

const SERVICE: &str = "net.hadess.SensorProxy";
const PATH: &str = "/net/hadess/SensorProxy/Compass";
const CLAIM_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_INITIAL: Duration = Duration::from_secs(1);
const RETRY_MAX: Duration = Duration::from_secs(30);

#[zbus::proxy(
    interface = "net.hadess.SensorProxy.Compass",
    default_service = "net.hadess.SensorProxy",
    default_path = "/net/hadess/SensorProxy/Compass"
)]
trait CompassSensor {
    fn claim_compass(&self) -> zbus::Result<()>;
    fn release_compass(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn has_compass(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn compass_heading(&self) -> zbus::Result<f64>;
}

#[derive(Clone, Debug)]
pub enum SensorEvent {
    Unavailable,
    Heading(f64),
    AccessDenied,
    Failed(String),
}

pub fn subscription() -> Subscription<SensorEvent> {
    struct SensorSubscription;
    Subscription::run_with(TypeId::of::<SensorSubscription>(), |_| {
        stream::channel(8, async |mut output| {
            let mut retry_delay = RETRY_INITIAL;
            loop {
                let error = match monitor(&mut output).await {
                    Ok(()) => "compass sensor stream ended".to_owned(),
                    Err(error) => error,
                };
                tracing::warn!(%error, service = SERVICE, path = PATH, "compass sensor failed; retrying");
                let event = if error.contains("AccessDenied") {
                    SensorEvent::AccessDenied
                } else {
                    SensorEvent::Failed(error)
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

async fn monitor(output: &mut futures::channel::mpsc::Sender<SensorEvent>) -> Result<(), String> {
    let connection = zbus::Connection::system()
        .await
        .map_err(|error| error.to_string())?;
    let proxy = CompassSensorProxy::new(&connection)
        .await
        .map_err(|error| error.to_string())?;

    // Subscribe before claiming so availability changes cannot race setup.
    // Property streams include their cached value as the first event.
    let mut availability = proxy.receive_has_compass_changed().await.fuse();
    let mut headings = proxy.receive_compass_heading_changed().await.fuse();
    let mut has_compass = proxy
        .has_compass()
        .await
        .map_err(|error| error.to_string())?;

    // Claim even while unavailable: this keeps the subscription alive if a
    // sensor appears later. Bound the reply so a broken provider cannot stall.
    match tokio::time::timeout(CLAIM_TIMEOUT, proxy.claim_compass()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(error.to_string()),
        Err(_) => {
            let _ = tokio::time::timeout(Duration::from_secs(1), proxy.release_compass()).await;
            return Err("ClaimCompass timed out after 5 seconds".to_owned());
        }
    }

    if !has_compass {
        let _ = output.send(SensorEvent::Unavailable).await;
    } else {
        emit_heading(proxy.compass_heading().await.ok(), output).await;
    }

    loop {
        select! {
            changed = availability.next() => {
                let Some(changed) = changed else { break };
                match changed.get().await {
                    Ok(value) => {
                        has_compass = value;
                        if has_compass {
                            emit_heading(proxy.compass_heading().await.ok(), output).await;
                        } else if output.send(SensorEvent::Unavailable).await.is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = proxy.release_compass().await;
                        return Err(error.to_string());
                    }
                }
            }
            changed = headings.next() => {
                let Some(changed) = changed else { break };
                if has_compass {
                    match changed.get().await {
                        Ok(value) => emit_heading(Some(value), output).await,
                        Err(error) => {
                            let _ = proxy.release_compass().await;
                            return Err(error.to_string());
                        }
                    }
                }
            }
        }
    }

    proxy
        .release_compass()
        .await
        .map_err(|error| error.to_string())
}

async fn emit_heading(
    value: Option<f64>,
    output: &mut futures::channel::mpsc::Sender<SensorEvent>,
) {
    match value.filter(|value| value.is_finite() && *value >= 0.0) {
        Some(value) => {
            let _ = output.send(SensorEvent::Heading(value)).await;
        }
        None => {
            let _ = output.send(SensorEvent::Unavailable).await;
        }
    }
}
