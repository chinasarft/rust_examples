#[cfg(test)]
mod extmap_test;

use crate::sdp::common::*;
use super::error::{Error, Result};
use super::direction::*;

use std::fmt;
use std::io;
use url::Url;
use std::collections::HashMap;

type ExtIdx = u32;

/// Default ext values
pub const DEF_EXT_MAP_VALUE_ABS_SEND_TIME: usize = 1;
pub const DEF_EXT_MAP_VALUE_TRANSPORT_CC: usize = 2;
pub const DEF_EXT_MAP_VALUE_SDES_MID: usize = 3;
pub const DEF_EXT_MAP_VALUE_SDES_RTP_STREAM_ID: usize = 4;

pub const NONE_EXT: &str = "";
pub const ABS_SEND_TIME_EXT: &str = "http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time";
pub const TRANSPORT_CC_EXT: &str = "http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01";
pub const PLAYOUT_DELAY_EXT: &str = "http://www.webrtc.org/experiments/rtp-hdrext/playout-delay";
pub const VIDEO_CONTENT_TYPE_EXT: &str = "http://www.webrtc.org/experiments/rtp-hdrext/video-content-type";
pub const VIDEO_TIMING_EXT: &str = "http://www.webrtc.org/experiments/rtp-hdrext/video-timing";
pub const COLOR_SPACE_EXT: &str = "http://www.webrtc.org/experiments/rtp-hdrext/color-space";
pub const SDES_MID_EXT: &str = "urn:ietf:params:rtp-hdrext:sdes:mid";
pub const SDES_RID_EXT: &str = "urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id";
pub const SDES_RRID_EXT: &str = "urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id";
pub const AUDIO_LEVEL_EXT: &str = "urn:ietf:params:rtp-hdrext:ssrc-audio-level";
pub const VIDEO_ORIENTATION_EXT: &str = "urn:3gpp:video-orientation";
pub const TOFFSET_EXT: &str = "urn:ietf:params:rtp-hdrext:toffset";

pub const ExtIdxNone: ExtIdx = 0;
pub const ABS_SEND_TIME_EXT_IDX: ExtIdx = 1;
pub const TRANSPORT_CC_EXT_IDX: ExtIdx = 2;
pub const PLAYOUT_DELAY_EXT_IDX: ExtIdx = 3;
pub const VIDEO_CONTENT_TYPE_EXT_IDX: ExtIdx = 4;
pub const VIDEO_TIMING_EXT_IDX: ExtIdx = 5;
pub const COLOR_SPACE_EXT_IDX: ExtIdx = 6;
pub const SDES_MID_EXT_IDX: ExtIdx = 7;
pub const SDES_RID_EXT_IDX: ExtIdx = 8;
pub const SDES_RRID_EXT_IDX: ExtIdx = 9; 
pub const AUDIO_LEVEL_EXT_IDX: ExtIdx = 10;
pub const VIDEO_ORIENTATION_EXT_IDX: ExtIdx = 11; 
pub const TOFFSET_EXT_IDX: ExtIdx = 12;

lazy_static! {
    static ref ext_url_idx_map: HashMap<&'static str, ExtIdx> = {
        let mut m = HashMap::new();
        m.insert(ABS_SEND_TIME_EXT, ABS_SEND_TIME_EXT_IDX);
        m.insert(TRANSPORT_CC_EXT, TRANSPORT_CC_EXT_IDX);
        m.insert(PLAYOUT_DELAY_EXT, PLAYOUT_DELAY_EXT_IDX);
        m.insert(VIDEO_CONTENT_TYPE_EXT, VIDEO_CONTENT_TYPE_EXT_IDX);
        m.insert(VIDEO_TIMING_EXT, VIDEO_TIMING_EXT_IDX);
        m.insert(COLOR_SPACE_EXT, COLOR_SPACE_EXT_IDX);
        m.insert(SDES_MID_EXT, SDES_MID_EXT_IDX);
        m.insert(SDES_RID_EXT, SDES_RID_EXT_IDX);
        m.insert(SDES_RRID_EXT, SDES_RRID_EXT_IDX);
        m.insert(AUDIO_LEVEL_EXT, AUDIO_LEVEL_EXT_IDX);
        m.insert(VIDEO_ORIENTATION_EXT, VIDEO_ORIENTATION_EXT_IDX);
        m.insert(TOFFSET_EXT, TOFFSET_EXT_IDX);
        m
    };
}

pub const ext_idx_url_map: [&'static str;13] = [
    NONE_EXT,
    ABS_SEND_TIME_EXT,
    TRANSPORT_CC_EXT,
    PLAYOUT_DELAY_EXT,
    VIDEO_CONTENT_TYPE_EXT,
    VIDEO_TIMING_EXT,
    COLOR_SPACE_EXT,
    SDES_MID_EXT,
    SDES_RID_EXT,
    SDES_RRID_EXT,
    AUDIO_LEVEL_EXT,
    VIDEO_ORIENTATION_EXT,
    TOFFSET_EXT,
];

/// ExtMap represents the activation of a single RTP header extension
#[derive(Debug, Clone, Default)]
pub struct ExtMap {
    pub value: isize,
    pub direction: Direction,
    pub uri_idx: ExtIdx,
    pub ext_attr: Option<String>,
}

impl fmt::Display for ExtMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = format!("{}", self.value);
        if self.direction != Direction::Unspecified {
            output += format!("/{}", self.direction).as_str();
        }

        let uri_str = get_ext_uri_by_idx(self.uri_idx);
        output += format!(" {}", uri_str).as_str();

        if let Some(ext_attr) = &self.ext_attr {
            output += format!(" {}", ext_attr).as_str();
        }

        write!(f, "{}", output)
    }
}

impl ExtMap {
    /// converts this object to an Attribute
    pub fn convert(&self) -> Attribute {
        Attribute {
            key: "extmap".to_string(),
            value: Some(self.to_string()),
        }
    }

    /// unmarshal creates an Extmap from a string
    pub fn unmarshal<R: io::BufRead>(reader: &mut R) -> Result<Self> {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let parts: Vec<&str> = line.trim().splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(Error::ParseExtMap(line));
        }

        let fields: Vec<&str> = parts[1].split_whitespace().collect();
        if fields.len() < 2 {
            return Err(Error::ParseExtMap(line));
        }

        let valdir: Vec<&str> = fields[0].split('/').collect();
        let value = valdir[0].parse::<isize>()?;
        if !(1..=246).contains(&value) {
            return Err(Error::ParseExtMap(format!(
                "{} -- extmap key must be in the range 1-256",
                valdir[0]
            )));
        }

        let mut direction = Direction::Unspecified;
        if valdir.len() == 2 {
            direction = Direction::new(valdir[1]);
            if direction == Direction::Unspecified {
                return Err(Error::ParseExtMap(format!(
                    "unknown direction from {}",
                    valdir[1]
                )));
            }
        }

        let uri_idx = get_idx_by_ext_uri(fields[1]);

        let ext_attr = if fields.len() == 3 {
            Some(fields[2].to_owned())
        } else {
            None
        };

        Ok(ExtMap {
            value,
            direction,
            uri_idx,
            ext_attr,
        })
    }

    /// marshal creates a string from an ExtMap
    pub fn marshal(&self) -> String {
        "extmap:".to_string() + self.to_string().as_str()
    }
}

pub fn get_idx_by_ext_uri(uri: &str) -> ExtIdx {
    let opt_idx =  ext_url_idx_map.get(uri);
    if let Some(idx) = opt_idx {
        return *idx;
    }
    ExtIdxNone
}

pub fn get_ext_uri_by_idx(idx: ExtIdx) -> &'static str {
    let uri = ext_idx_url_map.get(idx as usize);
    if let Some(uriStr) = uri {
        return uriStr;
    }
    NONE_EXT
}

