use crate::requests::PlaceRequestBody;
use axum::extract::ConnectInfo;
use axum::http::StatusCode;
use axum::Json;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::ops::{Add, Sub};
use std::os::unix::prelude::FileExt;
use std::panic::Location;
use std::sync::Mutex;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

const BOARD_FILE: &str = "board.txt";
const DIFF_FILE: &str = "diffs.bin";
const TIMEOUT: Duration = Duration::from_secs(5 * 60);

static HASH: Lazy<Mutex<HashMap<IpAddr, Duration>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn board() -> (StatusCode, String) {
    let file = match OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(BOARD_FILE)
    {
        Ok(o) => o,
        Err(e) => return error(e),
    };

    match BufReader::new(file).lines().next() {
        Some(Ok(o)) => (StatusCode::OK, o),
        Some(Err(e)) => error(e),
        None => error(std::io::Error::new(ErrorKind::NotFound, "Not found")),
    }
}

pub async fn submit(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Json<PlaceRequestBody>,
) -> (StatusCode, String) {
    let epoch = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(o) => o,
        Err(e) => return error(e),
    };

    let ip = match addr.ip() {
        IpAddr::V6(o) if is_global(&o) => {
            IpAddr::V6(mask_ipv6_host_identifier(o.segments()).into())
        }
        o => o,
    };

    let mut hash = match HASH.lock() {
        Ok(o) => o,
        Err(e) => return error(e),
    };

    if let Some(o) = hash.get(&ip) {
        if o.add(TIMEOUT) > epoch {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "Please wait {} seconds before placing again",
                    o.add(TIMEOUT).sub(epoch).as_secs()
                ),
            );
        }
    }

    hash.insert(ip, epoch);
    drop(hash);

    let board_file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(BOARD_FILE)
    {
        Ok(o) => o,
        Err(e) => return error(e),
    };

    let mut diff_file = match OpenOptions::new().append(true).create(true).open(DIFF_FILE) {
        Ok(o) => o,
        Err(e) => return error(e),
    };

    let len = match board_file.metadata() {
        Ok(m) => m.len(),
        Err(e) => return error(e),
    };

    if req.index as u64 >= len {
        return (
            StatusCode::BAD_REQUEST,
            format!("Index must be less than {len}"),
        );
    }

    if let Err(e) = board_file.write_at(&[req.pixel.to_byte()], req.index as u64) {
        return error(e);
    }

    let mut buffer = [0u8; 16];
    let mut writer = &mut buffer[..];
    let time = epoch.as_millis() as u64;
    const MESSAGE: &str = "Writing to a 16 byte buffer should never fail";

    writer.write(&time.to_be_bytes()).expect(MESSAGE);
    writer.write(&req.index.to_be_bytes()).expect(MESSAGE);
    writer.write(&[0, 0, 0, req.pixel.to_byte()]).expect(MESSAGE);

    if let Err(e) = diff_file.write(&buffer[..]) {
        return error(e);
    }

    (StatusCode::OK, "OK".to_string())
}

#[track_caller]
fn error<T: Error>(error: T) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!(
            "{}\nInternal server error at {}:{}",
            error,
            Location::caller().file(),
            Location::caller().line()
        ),
    )
}

const fn mask_ipv6_host_identifier(array: [u16; 8]) -> [u16; 8] {
    [array[0], array[1], array[2], array[3], 0, 0, 0, 0]
}

// Stolen from https://doc.rust-lang.org/src/core/net/ip_addr.rs.html
// since at the time of writing, it is unstable.
const fn is_global(ip: &Ipv6Addr) -> bool {
    !(ip.is_unspecified()
        || ip.is_loopback()
        // IPv4-mapped Address (`::ffff:0:0/96`)
        || matches!(ip.segments(), [0, 0, 0, 0, 0, 0xffff, _, _])
        // IPv4-IPv6 Translat. (`64:ff9b:1::/48`)
        || matches!(ip.segments(), [0x64, 0xff9b, 1, _, _, _, _, _])
        // Discard-Only Address Block (`100::/64`)
        || matches!(ip.segments(), [0x100, 0, 0, 0, _, _, _, _])
        // IETF Protocol Assignments (`2001::/23`)
        || (matches!(ip.segments(), [0x2001, b, _, _, _, _, _, _] if b < 0x200)
        && !(
        // Port Control Protocol Anycast (`2001:1::1`)
        u128::from_be_bytes(ip.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0001
            // Traversal Using Relays around NAT Anycast (`2001:1::2`)
            || u128::from_be_bytes(ip.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0002
            // AMT (`2001:3::/32`)
            || matches!(ip.segments(), [0x2001, 3, _, _, _, _, _, _])
            // AS112-v6 (`2001:4:112::/48`)
            || matches!(ip.segments(), [0x2001, 4, 0x112, _, _, _, _, _])
            // ORCHIDv2 (`2001:20::/28`)
            // Drone Remote ID Protocol Entity Tags (DETs) Prefix (`2001:30::/28`)`
            || matches!(ip.segments(), [0x2001, b, _, _, _, _, _, _] if b >= 0x20 && b <= 0x3F)
    ))
        // 6to4 (`2002::/16`) – it's not explicitly documented as globally reachable,
        // IANA says N/A.
        || matches!(ip.segments(), [0x2002, _, _, _, _, _, _, _])
        || (ip.segments()[0] == 0x2001) && (ip.segments()[1] == 0xdb8)
        || (ip.segments()[0] & 0xfe00) == 0xfc00
        || (ip.segments()[0] & 0xffc0) == 0xfe80)
}
