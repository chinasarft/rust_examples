use std::io::Cursor;

mod session;
mod time;
mod media;
mod common;
mod error;
mod direction;
pub mod extmap;

use session::*;
use time::*;
use media::*;
use error::*;

pub(crate) const END_LINE: &str = "\r\n";
pub(crate) const ATTRIBUTE_KEY: &str = "a=";

pub struct SDP {
    pub session: Session,

    /// <https://tools.ietf.org/html/rfc4566#section-5.9>
    /// <https://tools.ietf.org/html/rfc4566#section-5.10>
    pub time_descriptions: Vec<TimeDescription>,

    /// <https://tools.ietf.org/html/rfc4566#section-5.14>
    pub media_descriptions: Vec<MediaDescription>,
}


impl SDP {
    // pub fn new(literalSdp: &[u8]) -> Result<Self> {
    //     let mut reader = Cursor::new(literalSdp);

    //     return ()
    // }
}
