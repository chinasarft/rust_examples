#[cfg(test)]
mod sdp_test;

use std::io::Cursor;
use std::io;

mod session;
mod time;
mod media;
mod common;
mod error;
mod direction;
mod lexer;
pub mod extmap;

use common::*;
use session::*;
use time::*;
use media::*;
use error::*;
use lexer::*;
use url::Url;

pub struct SDP {
    pub session: Session,

    /// <https://tools.ietf.org/html/rfc4566#section-5.9>
    /// <https://tools.ietf.org/html/rfc4566#section-5.10>
    pub time_descriptions: Vec<TimeDescription>,

    /// <https://tools.ietf.org/html/rfc4566#section-5.14>
    pub media_descriptions: Vec<MediaDescription>,

    // todo datachannel
    // pub application: AppDescription,
}

//    Some lines in each description are REQUIRED and some are OPTIONAL,
//    but all MUST appear in exactly the order given here (the fixed order
//    greatly enhances error detection and allows for a simple parser).
//    OPTIONAL items are marked with a "*".
//       Session description
//          v=  (protocol version)
//          o=  (originator and session identifier)
//          s=  (session name)
//          i=* (session information)
//          u=* (URI of description)
//          e=* (email address)
//          p=* (phone number)
//          c=* (connection information -- not required if included in
//               all media)
//          b=* (zero or more bandwidth information lines)
//          One or more time descriptions ("t=" and "r=" lines; see below)
//          z=* (time zone adjustments)
//          k=* (encryption key)
//          a=* (zero or more session attribute lines)
//          Zero or more media descriptions

//       Time description
//          t=  (time the session is active)
//          r=* (zero or more repeat times)

//       Media description, if present
//          m=  (media name and transport address)
//          i=* (media title)
//          c=* (connection information -- optional if included at
//               session level)
//          b=* (zero or more bandwidth information lines)
//          k=* (encryption key)
//          a=* (zero or more media attribute lines)
//
// sdp的=*按顺序的*，然后参考这个代码的DFA状态机，代码的却比较简洁但是不是很好理解
// 所以打算重构了一下，确定各个状态的名字
// 状态的第一个字母s t m分别代表sdp的session time media部分
// 比如s函数，表示进入到session解析部分， t m类似
// 比如s_a,表示进入到session解析的a=部分
// 比如so,表示session可选的属性
// 优化后的状态机，初始化init, expect读取到v,进入到s_v状态
// s_s状态expectexpect i u e p c b t, 读取到expect的值后进入到对应状态
    /// +----+---------------------------------+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
    /// | No | STATES                          | v | o | s | i | u | e | p | c | b | t | r | z | k | a | m |
    /// +----+---------------------------------+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+
    /// | 1  |   s_v                           | 2 |   |   |   |   |   |   |   |   |   |   |   |   |   |   |
    /// | 2  |   s_o                           |   | 3 |   |   |   |   |   |   |   |   |   |   |   |   |   |
    /// | 3  |   s_s                           |   |   | 4 |   |   |   |   |   |   |   |   |   |   |   |   |
    /// | 4  |   so_iuepcbb_or_t_t             |   |   |   | 5 | 6 | 7 | 8 | 9 | 9 |10 |   |   |   |   |   |
    /// | 5  |   so_uepcbb_or_t_t              |   |   |   |   | 6 | 7 | 8 | 9 | 9 |10 |   |   |   |   |   |
    /// | 6  |   so_epcbb_or_t_t               |   |   |   |   |   | 7 | 8 | 9 | 9 |10 |   |   |   |   |   |
    /// | 7  |   so_pcbb_or_t_t                |   |   |   |   |   |   | 8 | 9 | 9 |10 |   |   |   |   |   |
    /// | 8  |   so_cbb_or_t_t                 |   |   |   |   |   |   |   | 9 | 9 |10 |   |   |   |   |   |
    /// | 9  |   so_bb_or_t_t                  |   |   |   |   |   |   |   |   | 9 |10 |   |   |   |   |   |
    /// | 10 |   to_rr_or_so_zkaa_or_m         |   |   |   |   |   |   |   |   |   |   |10 |11 |12 |12 |13 |
    /// | 11 |   so_kaa_or_m                   |   |   |   |   |   |   |   |   |   |   |   |   |12 |12 |13 |
    /// | 12 |   so_aa_or_m                    |   |   |   |   |   |   |   |   |   |   |   |   |   |12 |13 |
    /// | 13 |   moall                         |   |   |   |14 |   |   |   |15 |15 |   |   |   |16 |16 |13 |
    /// | 14 |   mo_cbbkaa                     |   |   |   |   |   |   |   |15 |15 |   |   |   |16 |16 |13 |
    /// | 15 |   mo_bbkaa                      |   |   |   |   |   |   |   |   |15 |   |   |   |16 |16 |13 |
    /// | 16 |   mo_aa                         |   |   |   |   |   |   |   |   |   |   |   |   |   |16 |13 |
    /// +----+---------------------------------+---+---+---+---+---+---+---+---+---+---+---+---+---+---+---+

impl SDP {
    pub fn unmarshal(literalSdp: &[u8]) -> Result<Self> {
        let mut reader = Cursor::new(literalSdp);

        let mut lexer = Lexer {
            sdp: SDP {
                session: Session::default(),
                time_descriptions: vec![],
                media_descriptions: vec![],
            },
            reader: &mut reader,
        };

        let mut state = Some(StateFn { f: s_v });
        while let Some(s) = state {
            state = (s.f)(&mut lexer)?;
        }

        Ok(lexer.sdp)
    }

    pub fn marshal(sdp: &SDP) -> String {
        let mut result = String::new();

        result += key_value_build("v=", Some(&sdp.session.version.to_string())).as_str();
        result += key_value_build("o=", Some(&sdp.session.origin.to_string())).as_str();
        result += key_value_build("s=", Some(&sdp.session.session_name)).as_str();

        result += key_value_build("i=", sdp.session.session_information.as_ref()).as_str();

        if let Some(uri) = &sdp.session.uri {
            result += key_value_build("u=", Some(&format!("{}", uri))).as_str();
        }
        result += key_value_build("e=", sdp.session.email_address.as_ref()).as_str();
        result += key_value_build("p=", sdp.session.phone_number.as_ref()).as_str();
        if let Some(connection_information) = &sdp.session.connection_information {
            result += key_value_build("c=", Some(&connection_information.to_string())).as_str();
        }

        for bandwidth in &sdp.session.bandwidth {
            result += key_value_build("b=", Some(&bandwidth.to_string())).as_str();
        }
        for time_description in &sdp.time_descriptions {
            result += key_value_build("t=", Some(&time_description.timing.to_string())).as_str();
            for repeat_time in &time_description.repeat_times {
                result += key_value_build("r=", Some(&repeat_time.to_string())).as_str();
            }
        }
        if !sdp.session.time_zones.is_empty() {
            let mut time_zones = vec![];
            for time_zone in &sdp.session.time_zones {
                time_zones.push(time_zone.to_string());
            }
            result += key_value_build("z=", Some(&time_zones.join(" "))).as_str();
        }
        result += key_value_build("k=", sdp.session.encryption_key.as_ref()).as_str();
        for attribute in &sdp.session.attributes {
            result += key_value_build("a=", Some(&attribute.to_string())).as_str();
        }

        for media_description in &sdp.media_descriptions {
            result +=
                key_value_build("m=", Some(&media_description.media_name.to_string())).as_str();
            result += key_value_build("i=", media_description.media_title.as_ref()).as_str();
            if let Some(connection_information) = &media_description.connection_information {
                result += key_value_build("c=", Some(&connection_information.to_string())).as_str();
            }
            for bandwidth in &media_description.bandwidth {
                result += key_value_build("b=", Some(&bandwidth.to_string())).as_str();
            }
            result += key_value_build("k=", media_description.encryption_key.as_ref()).as_str();
            for attribute in &media_description.attributes {
                result += key_value_build("a=", Some(&attribute.to_string())).as_str();
            }
        }

        result
    }
}

fn s_v<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, _) = read_type(lexer.reader)?;
    if &key == b"v=" {
        return Ok(Some(StateFn {
            f: unmarshal_protocol_version,
        }));
    }

    Err(Error::SdpInvalidSyntax(String::from_utf8(key)?))
}

fn s_o<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, _) = read_type(lexer.reader)?;
    if &key == b"o=" {
        return Ok(Some(StateFn {
            f: unmarshal_origin,
        }));
    }

    Err(Error::SdpInvalidSyntax(String::from_utf8(key)?))
}

fn s_s<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, _) = read_type(lexer.reader)?;
    if &key == b"s=" {
        return Ok(Some(StateFn {
            f: unmarshal_session_name,
        }));
    }

    Err(Error::SdpInvalidSyntax(String::from_utf8(key)?))
}

fn so_iuepcbb_or_t_t<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, _) = read_type(lexer.reader)?;
    match key.as_slice() {
        b"i=" => Ok(Some(StateFn {
            f: unmarshal_session_information,
        })),
        b"u=" => Ok(Some(StateFn { f: unmarshal_uri })),
        b"e=" => Ok(Some(StateFn { f: unmarshal_email })),
        b"p=" => Ok(Some(StateFn { f: unmarshal_phone })),
        b"c=" => Ok(Some(StateFn {
            f: unmarshal_session_connection_information,
        })),
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_session_bandwidth,
        })),
        b"t=" => Ok(Some(StateFn {
            f: unmarshal_timing,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn so_uepcbb_or_t_t<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, _) = read_type(lexer.reader)?;
    match key.as_slice() {
        b"u=" => Ok(Some(StateFn { f: unmarshal_uri })),
        b"e=" => Ok(Some(StateFn { f: unmarshal_email })),
        b"p=" => Ok(Some(StateFn { f: unmarshal_phone })),
        b"c=" => Ok(Some(StateFn {
            f: unmarshal_session_connection_information,
        })),
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_session_bandwidth,
        })),
        b"t=" => Ok(Some(StateFn {
            f: unmarshal_timing,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn so_epcbb_or_t_t<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, _) = read_type(lexer.reader)?;
    match key.as_slice() {
        b"e=" => Ok(Some(StateFn { f: unmarshal_email })),
        b"p=" => Ok(Some(StateFn { f: unmarshal_phone })),
        b"c=" => Ok(Some(StateFn {
            f: unmarshal_session_connection_information,
        })),
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_session_bandwidth,
        })),
        b"t=" => Ok(Some(StateFn {
            f: unmarshal_timing,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn so_pcbb_or_t_t<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, _) = read_type(lexer.reader)?;
    match key.as_slice() {
        b"p=" => Ok(Some(StateFn { f: unmarshal_phone })),
        b"c=" => Ok(Some(StateFn {
            f: unmarshal_session_connection_information,
        })),
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_session_bandwidth,
        })),
        b"t=" => Ok(Some(StateFn {
            f: unmarshal_timing,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn so_cbb_or_t_t<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, _) = read_type(lexer.reader)?;
    match key.as_slice() {
        b"c=" => Ok(Some(StateFn {
            f: unmarshal_session_connection_information,
        })),
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_session_bandwidth,
        })),
        b"t=" => Ok(Some(StateFn {
            f: unmarshal_timing,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn so_bb_or_t_t<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, _) = read_type(lexer.reader)?;
    match key.as_slice() {
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_session_bandwidth,
        })),
        b"t=" => Ok(Some(StateFn {
            f: unmarshal_timing,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn to_rr_or_so_zkaa_or_m<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, num_bytes) = read_type(lexer.reader)?;
    if key.is_empty() && num_bytes == 0 {
        return Ok(None);
    }

    match key.as_slice() {
        b"r=" => Ok(Some(StateFn {
            f: unmarshal_repeat_times,
        })),
        b"z=" => Ok(Some(StateFn {
            f: unmarshal_time_zones,
        })),
        b"k=" => Ok(Some(StateFn {
            f: unmarshal_session_encryption_key,
        })),
        b"a=" => Ok(Some(StateFn {
            f: unmarshal_session_attribute,
        })),
        // b"t=" => Ok(Some(StateFn { // TODO: remove
        //     f: unmarshal_timing,
        // })),
        b"m=" => Ok(Some(StateFn {
            f: unmarshal_media_description,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn so_kaa_or_m<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, num_bytes) = read_type(lexer.reader)?;
    if key.is_empty() && num_bytes == 0 {
        return Ok(None);
    }

    match key.as_slice() {
        b"k=" => Ok(Some(StateFn {
            f: unmarshal_session_encryption_key,
        })),
        b"a=" => Ok(Some(StateFn {
            f: unmarshal_session_attribute,
        })),
        b"m=" => Ok(Some(StateFn {
            f: unmarshal_media_description,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn so_aa_or_m<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, num_bytes) = read_type(lexer.reader)?;
    if key.is_empty() && num_bytes == 0 {
        return Ok(None);
    }

    match key.as_slice() {
        b"a=" => Ok(Some(StateFn {
            f: unmarshal_session_attribute,
        })),
        b"m=" => Ok(Some(StateFn {
            f: unmarshal_media_description,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn moall<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, num_bytes) = read_type(lexer.reader)?;
    if key.is_empty() && num_bytes == 0 {
        return Ok(None);
    }

    match key.as_slice() {
        b"i=" => Ok(Some(StateFn {
            f: unmarshal_media_title,
        })),
        b"c=" => Ok(Some(StateFn {
            f: unmarshal_media_connection_information,
        })),
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_media_bandwidth,
        })),
        b"k=" => Ok(Some(StateFn {
            f: unmarshal_media_encryption_key,
        })),
        b"a=" => Ok(Some(StateFn {
            f: unmarshal_media_attribute,
        })),
        b"m=" => Ok(Some(StateFn {
            f: unmarshal_media_description,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn mo_cbbkaa<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, num_bytes) = read_type(lexer.reader)?;
    if key.is_empty() && num_bytes == 0 {
        return Ok(None);
    }

    match key.as_slice() {
        b"c=" => Ok(Some(StateFn {
            f: unmarshal_media_connection_information,
        })),
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_media_bandwidth,
        })),
        b"k=" => Ok(Some(StateFn {
            f: unmarshal_media_encryption_key,
        })),
        b"a=" => Ok(Some(StateFn {
            f: unmarshal_media_attribute,
        })),
        // Non-spec ordering
        // b"i=" => Ok(Some(StateFn { // TODO: remove
        //     f: unmarshal_media_title,
        // })),
        b"m=" => Ok(Some(StateFn {
            f: unmarshal_media_description,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn mo_bbkaa<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, num_bytes) = read_type(lexer.reader)?;
    if key.is_empty() && num_bytes == 0 {
        return Ok(None);
    }

    match key.as_slice() {
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_media_bandwidth,
        })),
        b"k=" => Ok(Some(StateFn {
            f: unmarshal_media_encryption_key,
        })),
        b"a=" => Ok(Some(StateFn {
            f: unmarshal_media_attribute,
        })),
        // TODO: remove
        // b"c=" => Ok(Some(StateFn {
        //     f: unmarshal_media_connection_information,
        // })),
        // Non-spec ordering
        // b"i=" => Ok(Some(StateFn {
        //     f: unmarshal_media_title,
        // })),
        b"m=" => Ok(Some(StateFn {
            f: unmarshal_media_description,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn mo_aa<'a, R: io::BufRead + io::Seek>(lexer: &mut Lexer<'a, R>) -> Result<Option<StateFn<'a, R>>> {
    let (key, num_bytes) = read_type(lexer.reader)?;
    if key.is_empty() && num_bytes == 0 {
        return Ok(None);
    }

    match key.as_slice() {
        b"a=" => Ok(Some(StateFn {
            f: unmarshal_media_attribute,
        })),
        // Non-spec ordering
        b"k=" => Ok(Some(StateFn {
            f: unmarshal_media_encryption_key,
        })),
        // Non-spec ordering
        b"b=" => Ok(Some(StateFn {
            f: unmarshal_media_bandwidth,
        })),
        // Non-spec ordering
        b"c=" => Ok(Some(StateFn {
            f: unmarshal_media_connection_information,
        })),
        // Non-spec ordering
        b"i=" => Ok(Some(StateFn {
            f: unmarshal_media_title,
        })),
        b"m=" => Ok(Some(StateFn {
            f: unmarshal_media_description,
        })),
        _ => Err(Error::SdpInvalidSyntax(String::from_utf8(key)?)),
    }
}

fn unmarshal_protocol_version<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    let version = value.parse::<isize>()?;

    // As off the latest draft of the rfc this value is required to be 0.
    // https://tools.ietf.org/html/draft-ietf-rtcweb-jsep-24#section-5.8.1
    if version != 0 {
        return Err(Error::SdpInvalidSyntax(value));
    }
    lexer.sdp.session.version = version;

    Ok(Some(StateFn { f: s_o }))
}

fn unmarshal_origin<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    let fields: Vec<&str> = value.split_whitespace().collect();
    if fields.len() != 6 {
        return Err(Error::SdpInvalidSyntax(format!("`o={}`", value)));
    }

    let session_id = fields[1].parse::<u64>()?;
    let session_version = fields[2].parse::<u64>()?;

    // Set according to currently registered with IANA
    // https://tools.ietf.org/html/rfc4566#section-8.2.6
    let i = index_of(fields[3], &["IN"]);
    if i == -1 {
        return Err(Error::SdpInvalidValue(fields[3].to_owned()));
    }

    // Set according to currently registered with IANA
    // https://tools.ietf.org/html/rfc4566#section-8.2.7
    let i = index_of(fields[4], &["IP4", "IP6"]);
    if i == -1 {
        return Err(Error::SdpInvalidValue(fields[4].to_owned()));
    }

    // TODO validated UnicastAddress

    lexer.sdp.session.origin = Origin {
        username: fields[0].to_owned(),
        session_id,
        session_version,
        network_type: fields[3].to_owned(),
        address_type: fields[4].to_owned(),
        unicast_address: fields[5].to_owned(),
    };

    Ok(Some(StateFn { f: s_s }))
}

fn unmarshal_session_name<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;
    lexer.sdp.session.session_name = value;
    Ok(Some(StateFn { f: so_iuepcbb_or_t_t }))
}

fn unmarshal_session_information<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;
    lexer.sdp.session.session_information = Some(value);
    Ok(Some(StateFn { f: so_uepcbb_or_t_t }))
}

fn unmarshal_uri<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;
    lexer.sdp.session.uri = Some(Url::parse(&value)?);
    Ok(Some(StateFn { f: so_epcbb_or_t_t }))
}

fn unmarshal_email<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;
    lexer.sdp.session.email_address = Some(value);
    Ok(Some(StateFn { f: so_pcbb_or_t_t }))
}

fn unmarshal_phone<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;
    lexer.sdp.session.phone_number = Some(value);
    Ok(Some(StateFn { f: so_cbb_or_t_t }))
}

fn unmarshal_session_connection_information<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;
    lexer.sdp.session.connection_information = unmarshal_connection_information(&value)?;
    Ok(Some(StateFn { f: so_bb_or_t_t }))
}

fn unmarshal_connection_information(value: &str) -> Result<Option<ConnectionInformation>> {
    let fields: Vec<&str> = value.split_whitespace().collect();
    if fields.len() < 2 {
        return Err(Error::SdpInvalidSyntax(format!("`c={}`", value)));
    }

    // Set according to currently registered with IANA
    // https://tools.ietf.org/html/rfc4566#section-8.2.6
    let i = index_of(fields[0], &["IN"]);
    if i == -1 {
        return Err(Error::SdpInvalidValue(fields[0].to_owned()));
    }

    // Set according to currently registered with IANA
    // https://tools.ietf.org/html/rfc4566#section-8.2.7
    let i = index_of(fields[1], &["IP4", "IP6"]);
    if i == -1 {
        return Err(Error::SdpInvalidValue(fields[1].to_owned()));
    }

    let address = if fields.len() > 2 {
        Some(Address {
            address: fields[2].to_owned(),
            ttl: None,
            range: None,
        })
    } else {
        None
    };

    Ok(Some(ConnectionInformation {
        network_type: fields[0].to_owned(),
        address_type: fields[1].to_owned(),
        address,
    }))
}


fn unmarshal_session_bandwidth<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;
    lexer.sdp.session.bandwidth.push(unmarshal_bandwidth(&value)?);
    Ok(Some(StateFn { f: so_bb_or_t_t }))
}

fn unmarshal_bandwidth(value: &str) -> Result<Bandwidth> {
    let mut parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 2 {
        return Err(Error::SdpInvalidSyntax(format!("`b={}`", value)));
    }

    let experimental = parts[0].starts_with("X-");
    if experimental {
        parts[0] = parts[0].trim_start_matches("X-");
    } else {
        // Set according to currently registered with IANA
        // https://tools.ietf.org/html/rfc4566#section-5.8
        let i = index_of(parts[0], &["CT", "AS"]);
        if i == -1 {
            return Err(Error::SdpInvalidValue(parts[0].to_owned()));
        }
    }

    let bandwidth = parts[1].parse::<u64>()?;

    Ok(Bandwidth {
        experimental,
        bandwidth_type: parts[0].to_owned(),
        bandwidth,
    })
}


fn unmarshal_timing<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    let fields: Vec<&str> = value.split_whitespace().collect();
    if fields.len() < 2 {
        return Err(Error::SdpInvalidSyntax(format!("`t={}`", value)));
    }

    let start_time = fields[0].parse::<u64>()?;
    let stop_time = fields[1].parse::<u64>()?;

    lexer.sdp.time_descriptions.push(TimeDescription {
        timing: Timing {
            start_time,
            stop_time,
        },
        repeat_times: vec![],
    });

    Ok(Some(StateFn { f: to_rr_or_so_zkaa_or_m }))
}

fn unmarshal_repeat_times<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    let fields: Vec<&str> = value.split_whitespace().collect();
    if fields.len() < 3 {
        return Err(Error::SdpInvalidSyntax(format!("`r={}`", value)));
    }

    if let Some(latest_time_desc) = lexer.sdp.time_descriptions.last_mut() {
        let interval = parse_time_units(fields[0])?;
        let duration = parse_time_units(fields[1])?;
        let mut offsets = vec![];
        for field in fields.iter().skip(2) {
            let offset = parse_time_units(field)?;
            offsets.push(offset);
        }
        latest_time_desc.repeat_times.push(RepeatTime {
            interval,
            duration,
            offsets,
        });

        Ok(Some(StateFn { f: to_rr_or_so_zkaa_or_m }))
    } else {
        Err(Error::SdpEmptyTimeDescription)
    }
}

fn parse_time_units(value: &str) -> Result<i64> {
    // Some time offsets in the protocol can be provided with a shorthand
    // notation. This code ensures to convert it to NTP timestamp format.
    let val = value.as_bytes();
    let len = val.len();
    let (num, factor) = match val.last() {
        Some(b'd') => (&value[..len - 1], 86400), // days
        Some(b'h') => (&value[..len - 1], 3600),  // hours
        Some(b'm') => (&value[..len - 1], 60),    // minutes
        Some(b's') => (&value[..len - 1], 1),     // seconds (allowed for completeness)
        _ => (value, 1),
    };
    num.parse::<i64>()?
        .checked_mul(factor)
        .ok_or_else(|| Error::SdpInvalidValue(value.to_owned()))
}

fn unmarshal_time_zones<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    // These fields are transimitted in pairs
    // z=<adjustment time> <offset> <adjustment time> <offset> ....
    // so we are making sure that there are actually multiple of 2 total.
    let fields: Vec<&str> = value.split_whitespace().collect();
    if fields.len() % 2 != 0 {
        return Err(Error::SdpInvalidSyntax(format!("`t={}`", value)));
    }

    for i in (0..fields.len()).step_by(2) {
        let adjustment_time = fields[i].parse::<u64>()?;
        let offset = parse_time_units(fields[i + 1])?;

        lexer.sdp.session.time_zones.push(TimeZone {
            adjustment_time,
            offset,
        });
    }

    Ok(Some(StateFn { f: so_kaa_or_m }))
}

fn unmarshal_session_encryption_key<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;
    lexer.sdp.session.encryption_key = Some(value);
    Ok(Some(StateFn { f: so_aa_or_m }))
}

fn unmarshal_session_attribute<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    let fields: Vec<&str> = value.splitn(2, ':').collect();
    let attribute = if fields.len() == 2 {
        Attribute {
            key: fields[0].to_owned(),
            value: Some(fields[1].to_owned()),
        }
    } else {
        Attribute {
            key: fields[0].to_owned(),
            value: None,
        }
    };
    lexer.sdp.session.attributes.push(attribute);

    Ok(Some(StateFn { f: so_aa_or_m }))
}

fn unmarshal_media_description<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    let fields: Vec<&str> = value.split_whitespace().collect();
    if fields.len() < 4 {
        return Err(Error::SdpInvalidSyntax(format!("`m={}`", value)));
    }

    // <media>
    // Set according to currently registered with IANA
    // https://tools.ietf.org/html/rfc4566#section-5.14
    let i = index_of(
        fields[0],
        &["audio", "video", "text", "application", "message"],
    );
    if i == -1 {
        return Err(Error::SdpInvalidValue(fields[0].to_owned()));
    }

    // <port>
    let parts: Vec<&str> = fields[1].split('/').collect();
    let port_value = parts[0].parse::<u16>()? as isize;
    let port_range = if parts.len() > 1 {
        Some(parts[1].parse::<i32>()? as isize)
    } else {
        None
    };

    // <proto>
    // Set according to currently registered with IANA
    // https://tools.ietf.org/html/rfc4566#section-5.14
    let mut protos = vec![];
    for proto in fields[2].split('/').collect::<Vec<&str>>() {
        let i = index_of(
            proto,
            &[
                "UDP", "RTP", "AVP", "SAVP", "SAVPF", "TLS", "DTLS", "SCTP", "AVPF",
            ],
        );
        if i == -1 {
            return Err(Error::SdpInvalidValue(fields[2].to_owned()));
        }
        protos.push(proto.to_owned());
    }

    // <fmt>...
    let mut formats = vec![];
    for field in fields.iter().skip(3) {
        formats.push(field.to_string());
    }

    lexer.sdp.media_descriptions.push(MediaDescription {
        media_name: MediaName {
            media: fields[0].to_owned(),
            port: RangedPort {
                value: port_value,
                range: port_range,
            },
            protos,
            formats,
        },
        media_title: None,
        connection_information: None,
        bandwidth: vec![],
        encryption_key: None,
        attributes: vec![],
    });

    Ok(Some(StateFn { f: moall }))
}

fn unmarshal_media_title<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    if let Some(latest_media_desc) = lexer.sdp.media_descriptions.last_mut() {
        latest_media_desc.media_title = Some(value);
        Ok(Some(StateFn { f: mo_cbbkaa }))
    } else {
        Err(Error::SdpEmptyTimeDescription)
    }
}

fn unmarshal_media_connection_information<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    if let Some(latest_media_desc) = lexer.sdp.media_descriptions.last_mut() {
        latest_media_desc.connection_information = unmarshal_connection_information(&value)?;
        Ok(Some(StateFn { f: mo_bbkaa }))
    } else {
        Err(Error::SdpEmptyTimeDescription)
    }
}

fn unmarshal_media_bandwidth<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    if let Some(latest_media_desc) = lexer.sdp.media_descriptions.last_mut() {
        let bandwidth = unmarshal_bandwidth(&value)?;
        latest_media_desc.bandwidth.push(bandwidth);
        Ok(Some(StateFn { f: mo_bbkaa }))
    } else {
        Err(Error::SdpEmptyTimeDescription)
    }
}

fn unmarshal_media_encryption_key<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    if let Some(latest_media_desc) = lexer.sdp.media_descriptions.last_mut() {
        latest_media_desc.encryption_key = Some(value);
        Ok(Some(StateFn { f: mo_aa }))
    } else {
        Err(Error::SdpEmptyTimeDescription)
    }
}

fn unmarshal_media_attribute<'a, R: io::BufRead + io::Seek>(
    lexer: &mut Lexer<'a, R>,
) -> Result<Option<StateFn<'a, R>>> {
    let (value, _) = read_value(lexer.reader)?;

    let fields: Vec<&str> = value.splitn(2, ':').collect();
    let attribute = if fields.len() == 2 {
        Attribute {
            key: fields[0].to_owned(),
            value: Some(fields[1].to_owned()),
        }
    } else {
        Attribute {
            key: fields[0].to_owned(),
            value: None,
        }
    };

    if let Some(latest_media_desc) = lexer.sdp.media_descriptions.last_mut() {
        latest_media_desc.attributes.push(attribute);
        Ok(Some(StateFn { f: mo_aa }))
    } else {
        Err(Error::SdpEmptyTimeDescription)
    }
}



fn key_value_build(key: &str, value: Option<&String>) -> String {
    if let Some(val) = value {
        format!("{}{}{}", key, val, END_LINE)
    } else {
        "".to_string()
    }
}
