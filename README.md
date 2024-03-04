# MQTT Broker

[![MIT License](https://img.shields.io/badge/License-MIT-green.svg)](https://choosealicense.com/licenses/mit/)

An MQTT broker writen in rust that supports MQTT version 3.1.1

## Run Locally

Clone the project

```bash
  git clone https://github.com/VisualSource/Mqtt-Broker
```

Go to the project directory

```bash
  cd my-project
```

```bash
cargo run
```

## Running Tests

To run tests, run the following command

```bash
  cargo test
```

## SYS Topics

- `$SYS/broker/load/bytes/received`: The total number of bytes received since the broker started.

- `$SYS/broker/load/bytes/received`: The total number of bytes received since the broker started.

- `$SYS/broker/load/bytes/sent`: The total number of bytes sent since the broker started.

- `$SYS/broker/clients/connected`: The number of currently connected clients

- `$SYS/broker/clients/disconnected`: The total number of persistent clients (with clean session disabled) that are registered at the broker but are currently disconnected.

- `$SYS/broker/clients/maximum`: The maximum number of active clients that have been connected to the broker. This is only calculated when the $SYS topic tree is updated, so short lived client connections may not be counted.

- `$SYS/broker/clients/total`: The total number of connected and disconnected clients with a persistent session currently connected and registered on the broker.

- `$SYS/broker/messages/received`: The total number of messages of any type received since the broker started.

- `$SYS/broker/messages/sent`: The total number of messages of any type sent since the broker started.

- `$SYS/broker/messages/publish/dropped`: The total number of publish messages that have been dropped due to inflight/queuing limits.

- `$SYS/broker/messages/publish/received`: The total number of PUBLISH messages received since the broker started.

- `$SYS/broker/messages/publish/sent`: The total number of PUBLISH messages sent since the broker started.

- `$SYS/broker/messages/retained/count`: The total number of retained messages active on the broker.

- `$SYS/broker/subscriptions/count`: The total number of subscriptions active on the broker.

- `$SYS/broker/time`: The current time on the server.

- `$SYS/broker/uptime`: The amount of time in seconds the broker has been online.

- `$SYS/broker/version`: The version of the broker. Static.

## Authors

- [@VisualSource](https://www.github.com/visualsource)

## License

[MIT](https://choosealicense.com/licenses/mit/)

## Acknowledgements

- [Broker $SYS Topics](https://github.com/mqtt/mqtt.org/wiki/SYS-Topics)
- [MQTT Version 3.1.1 + Errata Specification](http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/mqtt-v3.1.1.html)
- [MQTT Version 5 Specification](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html)
- [C MQTT Broker Example](https://codepr.github.io/posts/sol-mqtt-broker/)
