mod config;
mod prometheus;

use crate::config::Config;
use color_eyre::{eyre::WrapErr, Result};
use rumqttc::{AsyncClient, Event, Outgoing, QoS};
use std::time::Duration;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config = Config::from_env()?;
    ctrlc::set_handler(move || {
        std::process::exit(0);
    })
    .wrap_err("Error setting Ctrl-C handler")?;

    let stall_detector = config.stall_detector();

    loop {
        for stalled in stall_detector
            .get_stalled(&config.metric, config.duration)
            .await?
        {
            info!("{} is stalled, resetting", stalled);
            send_mqtt_message(
                &config,
                format!("cmnd/{}/restart", stalled).into(),
                "1".into(),
            )
            .await?;
        }
        tokio::time::sleep(config.duration).await;
    }
}

#[tracing::instrument(skip(config))]
async fn send_mqtt_message(config: &Config, topic: String, payload: String) -> Result<()> {
    let mqtt_options = config.mqtt()?;

    let (mqtt_client, mut event_loop) = AsyncClient::new(mqtt_options, 10);
    mqtt_client
        .publish(topic, QoS::AtLeastOnce, false, payload)
        .await?;
    mqtt_client.disconnect().await?;

    if let Err(_) = tokio::time::timeout(Duration::from_secs(1), async move {
        while let Ok(event) = event_loop.poll().await {
            if matches!(event, Event::Outgoing(Outgoing::Disconnect)) {
                break;
            }
        }
    })
    .await
    {
        error!("Timeout while sending message")
    }
    info!("message successfully sent");
    Ok(())
}
