/// Config
/// See [Mosquitto](https://mosquitto.org/man/mosquitto-conf-5.html)
pub struct Config {
    user: Option<String>,
    pass: Option<String>,
    allow_anonymous: bool,

    address: String,
    host: usize,

    sys_interval: usize,
}
