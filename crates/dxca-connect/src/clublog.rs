//! ClubLog download client — the Rust counterpart of the Swift
//! `ClubLogClient`'s network half (the parse/matrix half lives in
//! dxca-core). Blocking (ureq); callers run it on a blocking task.
//!
//! Endpoints (1.x parity):
//!  - cty.xml:  GET  https://cdn.clublog.org/cty.php?api=<key>
//!  - ADIF log: POST https://clublog.org/getadif.php  (email/password/call)
//!
//! Both responses may be gzipped — detected by magic, like 1.x.

use std::io::Read;

/// Why a ClubLog request failed, with **403 called out separately**.
///
/// ClubLog's integration guidance asks that a 403 disable further requests
/// immediately rather than be retried: their reactive firewall watches for
/// repeated bad-credential traffic and blocks the source IP, which would take
/// out every ClubLog feature on the host — not just the one with the wrong
/// password. A `Result<_, String>` made every failure look the same, so a
/// wrong key and a flaky network were retried identically, on a timer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClubLogError {
    /// HTTP 403 — ClubLog rejected these credentials. Do not retry them.
    Forbidden(String),
    /// Anything else: timeouts, 5xx, a truncated body. Retrying is fine.
    Other(String),
}

impl ClubLogError {
    /// True when the credentials themselves were rejected, so retrying them
    /// unchanged is both useless and against ClubLog's asking.
    pub fn is_forbidden(&self) -> bool {
        matches!(self, ClubLogError::Forbidden(_))
    }
}

impl std::fmt::Display for ClubLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClubLogError::Forbidden(m) | ClubLogError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ClubLogError {}

impl From<String> for ClubLogError {
    fn from(m: String) -> Self {
        ClubLogError::Other(m)
    }
}

/// Classify a ureq failure, keeping 403 distinct from everything else.
fn classify(what: &str, e: ureq::Error) -> ClubLogError {
    match e {
        ureq::Error::Status(403, _) => ClubLogError::Forbidden(format!(
            "{what}: ClubLog rejected the credentials (HTTP 403)"
        )),
        other => ClubLogError::Other(format!("{what}: {other}")),
    }
}

/// Endpoint bases, overridable for tests (a fake local ClubLog).
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub cty_base: String,
    pub adif_base: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Endpoints {
            cty_base: "https://cdn.clublog.org".into(),
            adif_base: "https://clublog.org".into(),
        }
    }
}

impl Endpoints {
    /// Point both endpoints at one base — the test override.
    pub fn single_base(base: &str) -> Self {
        Endpoints {
            cty_base: base.trim_end_matches('/').into(),
            adif_base: base.trim_end_matches('/').into(),
        }
    }
}

pub fn download_cty(ep: &Endpoints, api_key: &str) -> Result<String, ClubLogError> {
    let url = format!("{}/cty.php?api={}", ep.cty_base, urlencode(api_key));
    let resp = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .map_err(|e| classify("CTY download", e))?;
    let bytes = read_body(resp)?;
    let bytes = gunzip_if_needed(bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn download_adif(
    ep: &Endpoints,
    callsign: &str,
    email: &str,
    password: &str,
) -> Result<Vec<u8>, ClubLogError> {
    let url = format!("{}/getadif.php", ep.adif_base);
    let body = format!(
        "email={}&password={}&call={}",
        urlencode(email),
        urlencode(password),
        urlencode(callsign)
    );
    let resp = ureq::post(&url)
        .timeout(std::time::Duration::from_secs(120))
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&body)
        .map_err(|e| classify("ADIF download", e))?;
    Ok(gunzip_if_needed(read_body(resp)?)?)
}

fn read_body(resp: ureq::Response) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    resp.into_reader()
        .take(256 * 1024 * 1024)
        .read_to_end(&mut out)
        .map_err(|e| format!("read body: {e}"))?;
    Ok(out)
}

/// 1.x check: gzip magic 0x1f 0x8b → decompress, else pass through.
fn gunzip_if_needed(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&bytes[..])
            .read_to_end(&mut out)
            .map_err(|e| format!("gunzip: {e}"))?;
        return Ok(out);
    }
    Ok(bytes)
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn gunzip_detection() {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(b"<clublog/>").unwrap();
        let gz = enc.finish().unwrap();
        assert_eq!(gunzip_if_needed(gz).unwrap(), b"<clublog/>");
        assert_eq!(gunzip_if_needed(b"plain".to_vec()).unwrap(), b"plain");
    }

    #[test]
    fn urlencoding() {
        assert_eq!(urlencode("a b+c@d"), "a%20b%2Bc%40d");
    }
}
