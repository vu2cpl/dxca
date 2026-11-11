//! ADIF mode string → award bucket ("CW" / "PHONE" / "DATA") — port of the
//! Swift `ModeNormalizer`. FT8, FT4, RTTY, JT65 etc. all collapse to DATA
//! so digital modes share one DXCC slot, matching DXCC/LoTW/ClubLog rules.

/// The three award buckets, in the order the UI shows them. Every value
/// `canonical` can return appears here — the test below is what keeps that
/// true if a fourth bucket is ever added.
pub const CLASSES: &[&str] = &["CW", "PHONE", "DATA"];

/// Settle a spot's mode: what the decoder or the spot comment reported wins;
/// otherwise fall back to inferring it from the frequency. Returns the mode
/// and whether it was inferred.
///
/// Exists because cluster nodes relaying human spots (DB0SUE, N2WQ) send
/// comments with no mode field at all. Before this, such a spot reached
/// `canonical("")` and was bucketed as **DATA** — an unmarked guess that
/// credited phone spots to digital award slots. An empty return means the
/// frequency was in no segment worth guessing about, and callers must treat
/// that as *unknown*, not as DATA.
/// WSJT-X reports a decode's mode as the **single character** it prints in
/// its own decode window, not as a mode name: `~` is FT8, `+` is FT4, and so
/// on. Passing that through unmapped puts a `~` in the mode column and makes
/// a locally decoded spot look like it has no mode at all — which is exactly
/// how it was reported from a Windows install running WSJT-X.
///
/// Only the characters worth being sure about are mapped. Anything else
/// returns `None`, so the caller can fall back to the Status message's own
/// mode string, which is a proper name and authoritative.
pub fn from_decoder_char(reported: &str) -> Option<&'static str> {
    let t = reported.trim();
    let mut chars = t.chars();
    let (c, rest) = (chars.next()?, chars.next());
    if rest.is_some() {
        return None; // a real name like "FT8", not a marker
    }
    match c {
        '~' => Some("FT8"),
        '+' => Some("FT4"),
        '#' => Some("JT65"),
        '@' => Some("JT9"),
        '&' => Some("MSK144"),
        ':' => Some("Q65"),
        _ => None,
    }
}

pub fn resolve(reported: &str, freq_mhz: f64) -> (String, bool) {
    let reported = reported.trim();
    if !reported.is_empty() {
        return (reported.to_string(), false);
    }
    match crate::bands::mode_from_mhz(freq_mhz) {
        Some(m) => (m.to_string(), true),
        None => (String::new(), false),
    }
}

/// Award bucket for an ADIF mode string.
///
/// **Empty input returns DATA**, which is right for the ADIF path it was
/// written for (a logged QSO always carries a mode; a blank one is a
/// malformed record) but wrong for a spot whose mode is simply unknown.
/// Spot callers use [`canonical_opt`] so unknown stays unknown.
pub fn canonical(raw: &str) -> &'static str {
    let mode = raw.trim().to_ascii_uppercase();
    if mode.is_empty() {
        return "DATA";
    }
    match mode.as_str() {
        "CW" => "CW",
        "SSB" | "USB" | "LSB" | "AM" | "FM" | "PHONE" | "VOICE" | "DIGITALVOICE" | "C4FM"
        | "DMR" | "DSTAR" => "PHONE",
        _ => "DATA",
    }
}

/// Beacon-only modes: WSPR and FST4W are one-way propagation beacons, so a
/// station spotted in one cannot be worked and must never raise an alert.
/// The spot itself still shows; it just never becomes a New DXCC/Band/Mode
/// or award catch. `WSPR` prefix also covers `WSPR-15` style names.
pub fn is_beacon_mode(raw: &str) -> bool {
    let mode = raw.trim().to_ascii_uppercase();
    mode.starts_with("WSPR") || mode == "FST4W"
}

/// Award bucket, or `None` when the mode is unknown. The spot path uses this
/// so an unknown mode is never silently scored as DATA.
pub fn canonical_opt(raw: &str) -> Option<&'static str> {
    if raw.trim().is_empty() {
        return None;
    }
    Some(canonical(raw))
}

#[cfg(test)]
mod tests {
    use super::{canonical, from_decoder_char, is_beacon_mode};

    #[test]
    fn wspr_and_fst4w_are_beacon_modes() {
        assert!(is_beacon_mode("WSPR"));
        assert!(is_beacon_mode(" wspr "));
        assert!(is_beacon_mode("WSPR-15"));
        assert!(is_beacon_mode("FST4W"));
        assert!(!is_beacon_mode("FST4"));
        assert!(!is_beacon_mode("FT8"));
        assert!(!is_beacon_mode(""));
    }

    /// The Windows/WSJT-X report: local spots showed no usable mode because
    /// the decoder sends a marker character, not a name.
    #[test]
    fn wsjtx_mode_characters_become_mode_names() {
        assert_eq!(from_decoder_char("~"), Some("FT8"));
        assert_eq!(from_decoder_char("+"), Some("FT4"));
        assert_eq!(from_decoder_char("#"), Some("JT65"));
        assert_eq!(from_decoder_char("@"), Some("JT9"));
        assert_eq!(from_decoder_char("&"), Some("MSK144"));
    }

    /// A decoder that sends a real name (MSHV sends "FT8") must pass
    /// straight through, and an unknown marker must not be guessed at.
    #[test]
    fn names_and_unknown_markers_are_left_alone() {
        assert_eq!(from_decoder_char("FT8"), None, "already a name");
        assert_eq!(from_decoder_char("SSB"), None);
        assert_eq!(from_decoder_char(""), None);
        assert_eq!(from_decoder_char("?"), None, "unmapped marker");
    }

    #[test]
    fn buckets() {
        assert_eq!(canonical("CW"), "CW");
        assert_eq!(canonical("ssb"), "PHONE");
        assert_eq!(canonical("FT8"), "DATA");
        assert_eq!(canonical("FT4"), "DATA");
        assert_eq!(canonical("RTTY"), "DATA");
        assert_eq!(canonical(" ft8 "), "DATA");
        assert_eq!(canonical(""), "DATA");
    }

    #[test]
    fn classes_cover_everything_canonical_can_return() {
        use super::CLASSES;
        for raw in [
            "CW",
            "SSB",
            "USB",
            "LSB",
            "AM",
            "FM",
            "PHONE",
            "VOICE",
            "DIGITALVOICE",
            "C4FM",
            "DMR",
            "DSTAR",
            "FT8",
            "FT4",
            "RTTY",
            "JT65",
            "MSK144",
            "",
            "nonsense",
        ] {
            let c = canonical(raw);
            assert!(CLASSES.contains(&c), "{raw} → {c}, which CLASSES omits");
        }
    }
}
