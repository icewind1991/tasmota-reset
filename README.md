# tasmota-reset

Automatically reset tasmota devices when a sensor stalls.

For use in combination with [taspromto](https://github.com/icewind1991/taspromto).

Detects when a sensor exported to prometheus has been reporting the same value for period of time
and automatically reboots the tasmota device when it happens.

## Usage

Run the binary with the following environment variables:

- `MQTT_HOSTNAME` hostname of the mqtt server
- `MQTT_USERNAME` username for the mqtt server (optional)
- `MQTT_PASSWORD` password for the mqtt server (optional)
- `PROMETHEUS_URL` url for the prometheus server
- `METRIC` metric to check for
- `DURATION` how long the sensor has to be stalled for to trigger the reset (defaults to `600`) 