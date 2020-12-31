use crate::prometheus::StallDetector;
use color_eyre::{eyre::WrapErr, Result};
use rumqttc::MqttOptions;
use std::str::FromStr;
use tokio::time::Duration;

#[derive(Default)]
pub struct Config {
    pub mqtt_host: String,
    pub mqtt_port: u16,
    pub prometheus_url: String,
    pub metric: String,
    pub mqtt_credentials: Option<Credentials>,
    pub duration: Duration,
}

pub struct Credentials {
    username: String,
    password: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mqtt_host = dotenv::var("MQTT_HOSTNAME").wrap_err("MQTT_HOSTNAME not set")?;
        let mqtt_port = dotenv::var("MQTT_PORT")
            .ok()
            .and_then(|port| u16::from_str(&port).ok())
            .unwrap_or(1883);

        let prometheus_url = dotenv::var("PROMETHEUS_URL").wrap_err("PROMETHEUS_URL not set")?;
        let metric = dotenv::var("METRIC").wrap_err("METRIC not set")?;

        let duration = match dotenv::var("DURATION") {
            Ok(duration) => duration.parse()?,
            Err(_) => 600,
        };
        let duration = Duration::from_secs(duration);

        let mqtt_credentials = match dotenv::var("MQTT_USERNAME") {
            Ok(username) => {
                let password = dotenv::var("MQTT_PASSWORD")
                    .wrap_err("MQTT_USERNAME set, but MQTT_PASSWORD not set")?;
                Some(Credentials { username, password })
            }
            Err(_) => None,
        };

        Ok(Config {
            mqtt_host,
            mqtt_port,
            prometheus_url,
            metric,
            mqtt_credentials,
            duration,
        })
    }

    pub fn mqtt(&self) -> Result<MqttOptions> {
        let pid = std::process::id();
        let mut mqtt_options = MqttOptions::new(
            format!("tasmota-reset-{}", pid),
            &self.mqtt_host,
            self.mqtt_port,
        );
        if let Some(credentials) = self.mqtt_credentials.as_ref() {
            mqtt_options.set_credentials(&credentials.username, &credentials.password);
        }
        mqtt_options.set_keep_alive(5);
        Ok(mqtt_options)
    }

    pub fn stall_detector(&self) -> StallDetector {
        StallDetector::new(&self.prometheus_url)
    }
}
