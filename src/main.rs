mod config;
mod prometheus;

use crate::config::Config;
use color_eyre::{eyre::WrapErr, Result};
use rumqttc::{AsyncClient, Event, Outgoing, QoS};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
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
            println!("{} is stalled, resetting", stalled);
            send_mqtt_message(
                &config,
                format!("cmnd/tasmota/{}/restart", stalled).into(),
                "1".into(),
            )
            .await?;
        }
        tokio::time::delay_for(config.duration).await;
    }
}

async fn send_mqtt_message(config: &Config, topic: String, payload: String) -> Result<()> {
    let mqtt_options = config.mqtt()?;

    let (mqtt_client, mut event_loop) = AsyncClient::new(mqtt_options, 10);
    mqtt_client
        .publish(topic, QoS::AtMostOnce, false, payload)
        .await?;
    mqtt_client.disconnect().await?;

    let _ = tokio::time::timeout(Duration::from_secs(1), async move {
        while let Ok(event) = event_loop.poll().await {
            if matches!(event, Event::Outgoing(Outgoing::Disconnect)) {
                break;
            }
        }
    })
    .await;
    Ok(())
}
