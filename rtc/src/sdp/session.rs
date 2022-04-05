use std::fmt;
use url::Url;
use crate::sdp::common::*;

#[derive(Debug, Default)]
pub struct Session {
    /// `v=0`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.1>
    pub version: isize,

    /// `o=<username> <sess-id> <sess-version> <nettype> <addrtype> <unicast-address>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.2>
    pub origin: Origin,

    /// `s=<session name>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.3>
    pub session_name: String,

    /// `i=<session description>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.4>
    pub session_information: Option<String>,

    /// `u=<uri>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.5>
    pub uri: Option<Url>,

    /// `e=<email-address>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.6>
    pub email_address: Option<String>,

    /// `p=<phone-number>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.6>
    pub phone_number: Option<String>,

    /// `c=<nettype> <addrtype> <connection-address>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.7>
    pub connection_information: Option<ConnectionInformation>,

    /// `b=<bwtype>:<bandwidth>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.8>
    pub bandwidth: Vec<Bandwidth>,

    /// `z=<adjustment time> <offset> <adjustment time> <offset> ...`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.11>
    pub time_zones: Vec<TimeZone>,

    /// `k=<method>`
    ///
    /// `k=<method>:<encryption key>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.12>
    pub encryption_key: Option<String>,

    /// `a=<attribute>`
    ///
    /// `a=<attribute>:<value>`
    ///
    /// <https://tools.ietf.org/html/rfc4566#section-5.13>
    pub attributes: Vec<Attribute>,
}

/// TimeZone defines the structured object for "z=" line which describes
/// repeated sessions scheduling.
#[derive(Debug, Default)]
pub struct TimeZone {
    pub adjustment_time: u64,
    pub offset: i64,
}

impl fmt::Display for TimeZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.adjustment_time, self.offset)
    }
}

/// Origin defines the structure for the "o=" field which provides the
/// originator of the session plus a session identifier and version number.
#[derive(Debug, Default)]
pub struct Origin {
    pub username: String,
    pub session_id: u64,
    pub session_version: u64,
    pub network_type: String,
    pub address_type: String,
    pub unicast_address: String,
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {}",
            self.username,
            self.session_id,
            self.session_version,
            self.network_type,
            self.address_type,
            self.unicast_address,
        )
    }
}

impl Origin {
    pub fn new() -> Self {
        Origin {
            username: "".to_owned(),
            session_id: 0,
            session_version: 0,
            network_type: "".to_owned(),
            address_type: "".to_owned(),
            unicast_address: "".to_owned(),
        }
    }
}
