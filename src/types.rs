use serde::{Deserialize, Serialize};

/// Representation of a USB event
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsbEvent {
    pub conn:     Option<String>,
    pub host:     Option<String>,
    pub vid:      Option<String>,
    pub pid:      Option<String>,
    pub prod:     Option<String>,
    pub manufact: Option<String>,
    pub serial:   Option<String>,
    pub port:     Option<String>,
    pub disconn:  Option<String>,
}

/// Methods for accessing USB event fields
impl UsbEvent {
    pub fn get_field(&self, key: &str) -> Option<&str> {
        match key {
            "conn"     => self.conn.as_deref(),
            "host"     => self.host.as_deref(),
            "vid"      => self.vid.as_deref(),
            "pid"      => self.pid.as_deref(),
            "prod"     => self.prod.as_deref(),
            "manufact" => self.manufact.as_deref(),
            "serial"   => self.serial.as_deref(),
            "port"     => self.port.as_deref(),
            "disconn"  => self.disconn.as_deref(),
            _          => None,
        }
    }
}

/// Options for filtering USB events
#[derive(Debug, Default, Clone)]
pub struct FilterOpts {
    pub date:     Vec<String>,
    pub host:     Vec<String>,
    pub vid:      Vec<String>,
    pub pid:      Vec<String>,
    pub prod:     Vec<String>,
    pub manufact: Vec<String>,
    pub serial:   Vec<String>,
    pub port:     Vec<String>,
    pub num:      Option<usize>,
}
