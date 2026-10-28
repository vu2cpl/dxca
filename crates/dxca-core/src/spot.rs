//! The aggregated spot — port of the Swift `SpotMessage` (minus the SwiftUI
//! display fields). Carries one decode (or a cluster spot converted to a
//! synthetic decode, 1.x-style) through dedupe, the telnet feed, UDP
//! broadcast, and classification.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spot {
    /// Unix seconds: today's UTC date carrying the decode's time-of-day
    /// (see [`time_from_decode_ms`]), or the receive time for cluster spots.
    pub time_unix: i64,
    pub snr_db: i32,
    pub delta_time_s: f64,
    pub delta_frequency_hz: u32,
    /// Raw mode exactly as the decoder sent it — 1.x passes Decode.mode
    /// through unmapped (WSJT-X uses mode characters like "~" for FT8),
    /// and the DATA bucket in the classifier absorbs whatever it is.
    pub mode: String,
    /// True when `mode` was **guessed from the frequency** rather than
    /// reported by the decoder or the spot comment.
    ///
    /// Cluster nodes that relay human spots (DB0SUE, N2WQ) send free-text
    /// comments with no mode field, and an empty mode used to be bucketed as
    /// DATA by `modes::canonical` — a silent, and often wrong, guess. It is
    /// still a guess now, but a labelled one: the UI marks it and the API
    /// exposes it, so an operator can see which award slots rest on an
    /// assumption. A decoder-reported mode always wins over an inferred one.
    pub mode_inferred: bool,
    pub message: String,
    /// Does this spot report a station **calling CQ**?
    ///
    /// Stored rather than sniffed from `message`, because for a cluster spot
    /// the message is synthesised and cannot carry the answer. Every cluster
    /// spot used to be built as `CQ <call>`, so the CQ-only filter matched
    /// 100% of the feed and appeared to do nothing.
    ///
    /// Cluster spots take it from the parsed `SpotKind`, widened to count
    /// skimmer spots: a skimmer only reports stations calling CQ, so an
    /// unmarked skimmer spot is one even though its comment never says so.
    /// A human spot with no marker is somebody logging a station they heard
    /// or worked, which is not.
    pub is_cq: bool,
    /// The spotter's free-text comment, for cluster spots — what a human
    /// actually typed. Empty for decoder spots, whose `message` already IS
    /// the decoded text.
    pub comment: String,
    pub low_confidence: bool,
    pub off_air: bool,
    pub dial_frequency_hz: u64,
    /// Where DXCA got this spot: a decoder source ("MSHV") or the configured
    /// name of the cluster node that relayed it ("DB0SUE", "HamAlert").
    ///
    /// **Not the same as who spotted it** — see [`Spot::spotter`]. A node
    /// name answers "which of my feeds carried this"; on a relaying node
    /// like HamAlert or DB0SUE that says nothing about the station whose
    /// receiver actually heard the DX.
    pub source_name: String,
    /// The **spotting station** — the call after `DX de` on the cluster
    /// line, skimmer `-#` suffix already stripped by the parser.
    ///
    /// `None` for spots decoded here, where the local receiver is the
    /// spotter and `source_name` already names it. The parser has always
    /// extracted this; until 2026-08-28 `synthetic_spot` dropped it on the
    /// floor, so every relayed spot was attributed to the relaying node and
    /// the operator could not tell a W3LPL skimmer catch from a hand-typed
    /// spot two hops away.
    #[serde(default)]
    pub spotter: Option<String>,
    /// The spotter was a **skimmer** — its callsign carried the `-#` marker
    /// the parser strips off [`Spot::spotter`].
    ///
    /// Kept because stripping the marker otherwise destroys the
    /// distinction: `W3LPL` and `W3LPL-#` are the same operator's station
    /// but not the same kind of spot, and an operator hunting real contacts
    /// wants the hand-typed ones. Always `false` for locally decoded spots,
    /// which have no spotter at all.
    #[serde(default)]
    pub is_skimmer: bool,
    /// The **DX station's** Maidenhead locator, when the spot carried one —
    /// a cluster comment's trailing grid (parsed by `wire.rs` and, until
    /// `docs/AWARDS.md` phase 2, dropped on this crate's doorstep), or the
    /// grid an FT8 CQ announces ([`grid_from_message`]). Uppercased, 4 or 6
    /// characters. This is what the VUCC classifier runs on.
    #[serde(default)]
    pub grid: Option<String>,
    /// The IOTA reference the spot announces (`AS-153`), normalized —
    /// extracted from a cluster comment by `awards::find_iota_ref`. Decoder
    /// spots never carry one: nothing in an FT8 exchange names an island.
    #[serde(default)]
    pub iota: Option<String>,
}

impl Spot {
    pub fn frequency_hz(&self) -> u64 {
        self.dial_frequency_hz + u64::from(self.delta_frequency_hz)
    }

    pub fn frequency_khz(&self) -> f64 {
        self.frequency_hz() as f64 / 1_000.0
    }

    pub fn frequency_mhz(&self) -> f64 {
        self.frequency_hz() as f64 / 1_000_000.0
    }

    // `is_cq` used to be derived here from the message text. It is a stored
    // field now — see the doc comment on it — because a synthesised cluster
    // message can only ever answer "yes".

    /// "HHmm" in UTC, for the cluster line.
    pub fn hhmm(&self) -> String {
        let secs = self.time_unix.rem_euclid(86_400);
        format!("{:02}{:02}", secs / 3600, (secs % 3600) / 60)
    }

    /// The spotted (DX) callsign extracted from the FT8/FT4 message text —
    /// Swift `SpotMessage.dxCallsign`, except that an unresolved-hash sender
    /// spots nobody (see below). Handles "CQ CALL GRID", directed
    /// "CQ NA CALL GRID", two-station exchanges (prefer the transmitting
    /// station in slot 1), and `<hashed>` calls.
    pub fn dx_callsign(&self) -> Option<String> {
        let parts: Vec<&str> = self.message.split(' ').filter(|p| !p.is_empty()).collect();
        if parts.len() < 2 {
            return None;
        }

        if parts[0].eq_ignore_ascii_case("CQ") {
            if parts.len() >= 3 && !looks_like_callsign(parts[1]) {
                return looks_like_callsign(parts[2]).then(|| strip_call_decoration(parts[2]));
            }
            return looks_like_callsign(parts[1]).then(|| strip_call_decoration(parts[1]));
        }

        // `CALL <...> RR73`: slot 1 is the sender, and it hashed its own
        // call — the decoder never resolved it. The one full call is the
        // station being CALLED, which may not be on the air at all; falling
        // back to it spotted (and alerted on) CP6CL at 7P8GE's offset,
        // because 7P8GE was finishing a QSO with him. The transmitter has no
        // name, so there is nobody to spot.
        if parts[1] == UNRESOLVED_HASH {
            return None;
        }
        if looks_like_callsign(parts[1]) {
            return Some(strip_call_decoration(parts[1]));
        }
        if looks_like_callsign(parts[0]) {
            return Some(strip_call_decoration(parts[0]));
        }
        None
    }

    /// The **DX station's audio offset** in Hz — where in the passband it is
    /// transmitting, the number you click in the waterfall to answer it.
    ///
    /// A decoder reports it directly, as `delta_frequency_hz`. A cluster spot
    /// arrives as a synthetic decode with that field at 0, so its offset can
    /// only come from the spotter's comment ([`offset_from_comment`]). `None`
    /// when neither says, never `Some(0)`: no FT8/FT4 signal sits at 0 Hz
    /// audio, and a 0 would send the operator to the bottom of the passband.
    ///
    /// Display only. Never write a comment's offset back into
    /// `delta_frequency_hz`: [`Spot::frequency_hz`] adds that field to the
    /// dial, and a skimmer that already spots at dial + offset (14075.8 for
    /// 14074 + 1794 Hz) would be counted twice, moving the spot's band, its
    /// dedupe key and every radio mark.
    pub fn dx_offset_hz(&self) -> Option<u32> {
        match self.delta_frequency_hz {
            0 => offset_from_comment(&self.comment),
            hz => Some(hz),
        }
    }

    /// Dedupe key: CALL-BAND-MODE (the 60-second-window key used both for
    /// display collapse and rebroadcast dedupe in 1.x). None when no
    /// callsign can be extracted — such spots always pass.
    pub fn duplicate_key(&self) -> Option<String> {
        let call = self.dx_callsign()?.to_uppercase();
        let band = crate::bands::band_from_hz(self.frequency_hz()).unwrap_or("");
        Some(format!("{call}-{band}-{}", self.mode.to_uppercase()))
    }
}

/// Does a decoded message announce a CQ? Only meaningful for real decoder
/// text — a synthesised cluster message can only ever say yes.
pub fn message_is_cq(message: &str) -> bool {
    message.to_uppercase().starts_with("CQ ")
}

/// The **transmitting station's** grid from decoded message text — the
/// trailing locator of `CQ K1JT FN20` or `K1ABC VU2XYZ MK83`. In both
/// standard forms the last token is sent by the transmitter, which is the
/// station [`Spot::dx_callsign`] extracts, so the grid belongs to the call.
///
/// `None` for reports, sign-offs and anything else: `grid::is_grid` refuses
/// `RR73` outright, and a message with no extractable callsign yields no
/// grid either — an award fact with no station to pin it on is noise.
pub fn grid_from_message(message: &str) -> Option<String> {
    let parts: Vec<&str> = message.split(' ').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let last = parts[parts.len() - 1];
    if !crate::grid::is_grid(last) {
        return None;
    }
    let probe = Spot {
        message: message.to_string(),
        ..blank_spot()
    };
    probe.dx_callsign().map(|_| last.to_ascii_uppercase())
}

/// The top of WSJT-X's passband. An offset above it is not an offset.
const MAX_OFFSET_HZ: u32 = 5_000;

/// The floor for an **unlabelled** offset. In the `-18 dB 6 FT8 2167` shape
/// (RBN Aggregator's, VU2OY's node) the field between dB and the mode is
/// the symbol rate in baud — `6` for FT8's 6.25, where a CW spot shows its
/// WPM — not an offset, and this floor is what stops the dB-mode rule
/// reading it as one. No skimmer offset in the 2026-10-09 sample was below
/// 185 Hz.
const MIN_UNLABELLED_OFFSET_HZ: u32 = 100;

/// The DX station's audio offset from a cluster comment, in Hz.
///
/// Three shapes, all from the shack's own feed (2,000 spots on .109,
/// 2026-10-09; 1,932 of the 1,936 FT8 cluster spots carried one):
///
/// - `FT8 1500Hz BL11`: labelled, attached or spaced (`1500 Hz`). The only
///   form a human types. `kHz` is not `Hz`, so `QSX up 2 kHz` stays out.
/// - `-15 dB 1032 FT8`: unlabelled, between the dB and the mode. That skimmer
///   spots at dial + offset (14075.0 for 14074 + 1032 Hz), and every spot in
///   the sample agreed with its frequency that way.
/// - `-18 dB 6 FT8 2167`, `-13 dB 6 FT8 CQ KN34 1497`: unlabelled, last. That
///   skimmer spots at the dial itself, so the comment is the only place the
///   offset survives.
///
/// The unlabelled shapes are read only inside their exact frame, an SNR,
/// `dB`, one field, then `FT8` or `FT4`. A bare number anywhere else could
/// be a power, a serial number or a time, and RBN's `FT8 -5 dB CQ` puts the
/// mode first, so it never enters the frame at all.
pub fn offset_from_comment(comment: &str) -> Option<u32> {
    let tokens: Vec<&str> = comment.split_whitespace().collect();
    labelled_offset(&tokens).or_else(|| framed_offset(&tokens))
}

/// `1500Hz`, `+1500Hz` or `1500 Hz`.
fn labelled_offset(tokens: &[&str]) -> Option<u32> {
    let clean = |t: &str| t.trim_end_matches([',', ';', '.', ')']).to_string();
    for (i, raw) in tokens.iter().enumerate() {
        let t = clean(raw.trim_start_matches('+'));
        let digits = t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if digits == 0 {
            continue;
        }
        let (num, unit) = t.split_at(digits);
        // `1.5kHz` splits as `1` + `.5kHz`: not a bare `Hz`, so not ours.
        let is_hz = if unit.is_empty() {
            tokens
                .get(i + 1)
                .is_some_and(|next| clean(next).eq_ignore_ascii_case("hz"))
        } else {
            unit.eq_ignore_ascii_case("hz")
        };
        if !is_hz {
            continue;
        }
        if let Some(hz) = num
            .parse::<u32>()
            .ok()
            .filter(|hz| (1..=MAX_OFFSET_HZ).contains(hz))
        {
            return Some(hz);
        }
    }
    None
}

/// The two skimmer shapes: `<snr> dB <n> FT8 …`, offset at `<n>` or last.
fn framed_offset(tokens: &[&str]) -> Option<u32> {
    let mode_at = tokens
        .iter()
        .position(|t| t.eq_ignore_ascii_case("FT8") || t.eq_ignore_ascii_case("FT4"))?;
    let framed = mode_at >= 3
        && tokens[mode_at - 2].eq_ignore_ascii_case("dB")
        && tokens[mode_at - 3].parse::<i32>().is_ok();
    if !framed {
        return None;
    }
    let plausible = |t: &str| {
        t.parse::<u32>()
            .ok()
            .filter(|hz| (MIN_UNLABELLED_OFFSET_HZ..=MAX_OFFSET_HZ).contains(hz))
    };
    let last = tokens.len() - 1;
    plausible(tokens[mode_at - 1]).or_else(|| {
        if last > mode_at {
            plausible(tokens[last])
        } else {
            None
        }
    })
}

/// A structurally valid spot with nothing in it — the base for
/// [`grid_from_message`]'s parse probe and the test constructors.
fn blank_spot() -> Spot {
    Spot {
        time_unix: 0,
        snr_db: 0,
        delta_time_s: 0.0,
        delta_frequency_hz: 0,
        mode: String::new(),
        mode_inferred: false,
        message: String::new(),
        is_cq: false,
        comment: String::new(),
        low_confidence: false,
        off_air: false,
        dial_frequency_hz: 0,
        source_name: String::new(),
        spotter: None,
        is_skimmer: false,
        grid: None,
        iota: None,
    }
}

/// Map a WSJT-X decode time (ms since midnight UTC) onto today's UTC date —
/// the Swift `timeFromMilliseconds` without a hidden clock: `now_unix`
/// supplies "today".
pub fn time_from_decode_ms(now_unix: i64, decode_ms: u32) -> i64 {
    let midnight = now_unix - now_unix.rem_euclid(86_400);
    midnight + i64::from(decode_ms / 1000)
}

/// What WSJT-X / JTDX / MSHV print for a hashed callsign they cannot resolve.
const UNRESOLVED_HASH: &str = "<...>";

/// Strip the `<>` brackets WSJT-X puts around hashed/known callsigns.
fn strip_call_decoration(s: &str) -> String {
    s.trim_start_matches('<').trim_end_matches('>').to_string()
}

/// Reject FT8 tokens that aren't callsigns — RR73/73/TU…, signal reports
/// (R+05, -12), 4-char Maidenhead grids (LL85), placeholders. Swift
/// `looksLikeCallsign` verbatim.
fn looks_like_callsign(s: &str) -> bool {
    let upper = s.to_uppercase();
    let core = upper.trim_start_matches('<').trim_end_matches('>');
    if core.is_empty() || core == "..." {
        return false;
    }
    let len = core.chars().count();
    if !(3..=11).contains(&len) {
        return false;
    }

    const BLACKLIST: [&str; 9] = ["RR73", "RRR", "73", "TU", "TNX", "QSL", "DE", "TEST", "CQ"];
    if BLACKLIST.contains(&core) {
        return false;
    }

    // Signal reports: R+05, R-12, +05, -12.
    if core.starts_with("R+") || core.starts_with("R-") {
        return false;
    }
    if (core.starts_with('+') || core.starts_with('-'))
        && core.chars().skip(1).all(|c| c.is_numeric())
    {
        return false;
    }

    // 4-char Maidenhead grid: 2 letters + 2 digits.
    if len == 4 {
        let c: Vec<char> = core.chars().collect();
        if c[0].is_alphabetic() && c[1].is_alphabetic() && c[2].is_numeric() && c[3].is_numeric() {
            return false;
        }
    }

    let has_digit = core.chars().any(|c| c.is_numeric());
    let has_letter = core.chars().any(|c| c.is_alphabetic());
    if !has_digit || !has_letter {
        return false;
    }

    core.chars()
        .all(|c| c.is_alphabetic() || c.is_numeric() || c == '/')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot(message: &str) -> Spot {
        Spot {
            time_unix: 1_787_745_000,
            snr_db: -12,
            delta_time_s: 0.2,
            delta_frequency_hz: 1487,
            mode: "FT8".into(),
            mode_inferred: false,
            message: message.into(),
            is_cq: true,
            dial_frequency_hz: 14_074_000,
            source_name: "JTDX".into(),
            ..super::blank_spot()
        }
    }

    #[test]
    fn dx_call_extraction_table() {
        // (message, expected dx callsign)
        let cases = [
            ("CQ P5DX PM95", Some("P5DX")),
            ("CQ NA K1JT FN20", Some("K1JT")), // directed CQ
            ("CQ DX VU2CPL MK83", Some("VU2CPL")),
            ("K1JT VU2CPL -15", Some("VU2CPL")), // exchange: prefer slot 1
            ("VU2CPL K1JT RR73", Some("K1JT")),
            ("K1JT RR73", Some("K1JT")), // slot 1 is decoration → fall back to slot 0
            ("<K1JT> VU2CPL R-07", Some("VU2CPL")),
            ("VU2CPL <K1JT> +03", Some("K1JT")), // hashed call unwraps
            ("K1JT LL85", Some("K1JT")),         // slot 1 is a grid → fall back to slot 0
            ("<...> W9HT/VP9 RRR", Some("W9HT/VP9")), // hashed addressee, full sender
            ("CQ TEST", None),
            ("CQ EU", None), // VU2OY's Aggregator spotted `EU` as a call
            ("TNX 73 GL", None),
            ("73", None),
        ];
        for (msg, want) in cases {
            assert_eq!(spot(msg).dx_callsign().as_deref(), want, "message: {msg:?}");
        }
    }

    /// The sender hashed its own call and the decoder never resolved it, so
    /// the only full call is the one being CALLED. Spotting it put CP6CL on
    /// the air (JTDX, 2026-10-20 01:17:30, `CP6CL <...> RR73` at 408 Hz) when
    /// the station at 408 Hz was 7P8GE, and W9HT/VP9 at every caller's 73.
    #[test]
    fn an_unresolved_hash_sender_spots_nobody() {
        for msg in [
            "CP6CL <...> RR73",
            "W9HT/VP9 <...> 73",
            "JM1LRA <...> -15",
            "JM1LRA <...> R-15",
            "CP6CL <...> FN42",
            "KP3DR/9 <...>",
        ] {
            assert_eq!(spot(msg).dx_callsign(), None, "message: {msg:?}");
            assert_eq!(spot(msg).duplicate_key(), None, "message: {msg:?}");
            // A locator in it is the unnamed sender's, not the addressee's.
            assert_eq!(grid_from_message(msg), None, "message: {msg:?}");
        }
    }

    #[test]
    fn message_grids_belong_to_the_transmitter() {
        let cases = [
            ("CQ P5DX PM95", Some("PM95")),
            ("CQ NA K1JT FN20", Some("FN20")),
            ("K1ABC VU2XYZ MK83", Some("MK83")), // std msg: grid is the sender's
            ("K1ABC VU2XYZ mk83va", Some("MK83VA")), // 6-char, uppercased
            ("VU2CPL K1JT RR73", None),          // sign-off, never a grid
            ("K1JT VU2CPL -15", None),           // report
            ("CQ TEST", None),                   // no extractable callsign
            ("CQ P5DX", None),                   // no grid at all
        ];
        for (msg, want) in cases {
            assert_eq!(grid_from_message(msg).as_deref(), want, "message: {msg:?}");
        }
    }

    #[test]
    fn cq_and_keys() {
        let s = spot("CQ P5DX PM95");
        assert!(s.is_cq);
        assert_eq!(s.duplicate_key().as_deref(), Some("P5DX-20M-FT8"));
        assert_eq!(spot("73").duplicate_key(), None);
        assert_eq!(s.frequency_hz(), 14_075_487);
    }

    /// Every shape the shack's feed carried on 2026-10-09, and the near
    /// misses that must not be read as offsets. The values come from real
    /// spots; the skimmer ones agree with their spot frequencies.
    #[test]
    fn offset_from_comment_table() {
        let cases: &[(&str, Option<u32>)] = &[
            // Labelled, typed by a human.
            ("FT8 1500Hz BL11", Some(1500)),
            ("FT8 1500 Hz BL11", Some(1500)),
            ("FT8 +1500hz", Some(1500)),
            ("ft8 tnx 850 HZ, 73", Some(850)),
            // Skimmer at dial + offset: offset between dB and the mode.
            ("-15 dB 1032 FT8", Some(1032)),
            ("-16 dB 245 FT8", Some(245)),
            ("-19 dB 1768 FT8 CQ KN34", Some(1768)),
            ("3 dB 2302 FT4", Some(2302)),
            // Skimmer at the dial: a single digit, then the offset last.
            ("-18 dB 6 FT8 2167", Some(2167)),
            ("-13 dB 6 FT8 635", Some(635)),
            ("-13 dB 6 FT8 CQ KN34 1497", Some(1497)),
            ("0 dB 6 FT8 2761", Some(2761)),
            // Not offsets.
            ("", None),
            ("TNX/FT8 OI51", None),
            ("FT8 USA250 NM", None),
            ("QSX up 2 kHz", None),
            ("FT8 1.5kHz up", None),
            ("FT8 -5 dB CQ", None), // RBN: mode first, outside the frame
            ("-13 dB 6 FT8 CQ KN34", None), // the single digit is not an offset
            ("CW 23 dB 25 WPM CQ", None), // a CW skimmer's speed
            ("-12 dB 9000 FT8", None), // above the passband
            ("FT8 0Hz", None),
        ];
        for (comment, want) in cases {
            assert_eq!(offset_from_comment(comment), *want, "{comment:?}");
        }
    }

    /// The decoder's own reading wins; the comment is only for cluster
    /// spots, which arrive with the field at 0.
    #[test]
    fn dx_offset_prefers_the_decoder_and_falls_back_to_the_comment() {
        assert_eq!(spot("CQ K1JT FN20").dx_offset_hz(), Some(1487));

        let cluster = Spot {
            delta_frequency_hz: 0,
            comment: "-11 dB 1794 FT8".into(),
            dial_frequency_hz: 14_075_800,
            ..spot("CQ K1JT")
        };
        assert_eq!(cluster.dx_offset_hz(), Some(1794));
        // Display only: the spot stays where the skimmer put it.
        assert_eq!(cluster.frequency_hz(), 14_075_800);

        let silent = Spot {
            comment: "TNX/FT8 OI51".into(),
            ..cluster
        };
        assert_eq!(silent.dx_offset_hz(), None);
    }

    #[test]
    fn decode_time_maps_onto_today() {
        // 2026-08-27 ~13:10 UTC; decode said 05:31:30 (19,890,000 ms).
        let t = time_from_decode_ms(1_787_836_200, 19_890_000);
        assert_eq!(t.rem_euclid(86_400), 5 * 3600 + 31 * 60 + 30);
        let s = Spot {
            time_unix: t,
            ..spot("CQ P5DX PM95")
        };
        assert_eq!(s.hhmm(), "0531");
    }
}
