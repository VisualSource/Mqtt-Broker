use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

pub struct ConfigBuilder {
    username: Option<String>,
    password: Option<String>,
    allow_anonymous: bool,
    address: String,
    port: u16,
    sys_interval: u64,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
            username: None,
            password: None,
            allow_anonymous: true,
            address: "0.0.0.0".into(),
            port: 1833,
            sys_interval: 10,
        }
    }

    /*pub fn set_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    pub fn set_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    pub fn set_allow_anonymous(mut self, anonymous: bool) -> Self {
        self.allow_anonymous = anonymous;
        self
    }

    pub fn set_address(mut self, address: String) -> Self {
        self.address = address;
        self
    }*/

    pub fn set_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn set_sys_interval(mut self, interval: u64) -> Self {
        self.sys_interval = interval;
        self
    }

    pub fn build(self) -> Result<Config, ()> {
        let host = IpAddr::from_str(&self.address).map_err(|_| ())?;

        let address = SocketAddr::new(host, self.port);

        Ok(Config {
            user: self.username,
            pass: self.password,
            allow_anonymous: self.allow_anonymous,
            socket_addr: address,
            sys_interval: self.sys_interval,
        })
    }
}

/// Config
/// See [Mosquitto](https://mosquitto.org/man/mosquitto-conf-5.html)
pub struct Config {
    pub user: Option<String>,
    pub pass: Option<String>,
    pub allow_anonymous: bool,

    pub socket_addr: SocketAddr,

    pub sys_interval: u64,
}
