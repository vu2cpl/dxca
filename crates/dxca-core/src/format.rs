//! DX-cluster announcement line — port of the Swift `ClusterFormatter`.
//! Mirrors the de-facto DX-Spider layout every cluster client tokenises:
//!
//! ```text
//! DX de MSHV:      14074.0  K1JT           -7 dB   6 FT8  CQ FN20 1758 1428Z
//! ```
//!
//! Parsers tokenise by whitespace runs, but the uppercase `Z`, the
//! unpadded frequency, and the padded cells are all deliberate — RUMlog
//! regex-matches `\d{4}Z$` and mis-parsed drifting columns in early 1.x.

use crate::spot::Spot;

pub fn format(spot: &Spot) -> String {
    let dx_call = spot
        .dx_callsign()
        .unwrap_or_else(|| "UNKNOWN".into())
        .to_uppercase();

    // Who heard it. A relayed cluster spot names its real spotter, with a
    // skimmer's `-#` put back the way DX-Spider and RBN send it, so a
    // logger shows `W3LPL-#` rather than the node that relayed it. Until
    // v2.24.3 this was always the source name, and RUMlog's "DX de" read
    // N2WQ-2 or W3LPL for every spot that node passed on (Manoj: "real de
    // is missing"). The node is still in the payload's `source_name` and
    // the web UI. A local decode has no spotter: its source is the receiver.
    //
    // The spotter must be a SINGLE token: source names like "MSHV 2237"
    // would tokenise as two fields and shove the freq into the call slot.
    // The `-#` goes on after cleaning, since `#` is not a callsign character.
    let clean = |s: &str| -> String {
        s.to_uppercase()
            .chars()
            .filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '/' || *c == '-')
            .collect()
    };
    let cleaned = match spot.spotter.as_deref().map(clean) {
        Some(call) if !call.is_empty() && spot.is_skimmer => format!("{call}-#"),
        Some(call) if !call.is_empty() => call,
        _ => clean(&spot.source_name),
    };
    let raw = if cleaned.is_empty() {
        "NOCALL".to_string()
    } else {
        cleaned
    };
    let spotter: String = raw.chars().take(13).collect(); // DX-Spider spotter limit

    // The frequency cell is the dial the spot was heard on. For a relayed
    // cluster spot that is its spotted frequency (delta 0). A decoder's spot
    // used to go out at dial + offset (14075.8 for 14074 + 1758 Hz); in the
    // Aggregator shape below the offset rides in the comment, relative to
    // the dial, so the cell is the dial — the frequency to put the rig on.
    // Both in one line would be counted twice by a logger reading the
    // comment, the hazard `Spot::dx_offset_hz` describes from the other
    // side.
    let freq_str = format!("{:.1}", spot.dial_frequency_hz as f64 / 1_000.0);
    let comment = comment_for(spot);

    format!(
        "DX de {} {} {}{} {}Z",
        pad_or_trunc(&format!("{spotter}:"), 14),
        pad_or_trunc(&freq_str, 9),
        pad_or_trunc(&dx_call, 14),
        pad_or_trunc(&comment, 28),
        spot.hhmm()
    )
}

/// The comment cell.
///
/// A relayed cluster spot's comment goes out exactly as it came in, even
/// when that is nothing. That text is what the loggers were written to read
/// — a skimmer's, RBN's, a human's `FT8 1500Hz BL11` with its grid — and
/// the 1.x rewrite to `FT8 -15 dB` threw the offset away, while a labelled
/// `DF 1032 Hz` (tried and dropped the same evening) was a form of this
/// program's invention. Manoj, 2026-10-09: "keep the original comment and
/// no need for any DF or Hz in comments. the logging softwares are made to
/// take it that way."
///
/// A decoder's spot has no comment, so one is made in the shape RBN
/// Aggregator gives an FT8 skimmer spot — VU2OY's node
/// (`vu2oy.ddns.net:7550`, "de SKIMMER via Aggregator") is one, and that
/// shape is what the loggers around here read (Manoj: "sequence it exactly
/// like vu2oy format"):
///
/// ```text
/// DX de VU2OY-#:   14074.0  YC2VTS         -7 dB   6 FT8          1758  0607Z
/// DX de VU2OY-#:   28074.0  UN7LZ          -6 dB   6 FT8  CQ MO13 2332  0607Z
/// DX de VU2OY-#:   18100.0  UW5KW         -19 dB   6 FT8  CQ      1364  0607Z
/// ```
///
/// SNR right-aligned in 3 and ` dB`; the symbol rate in baud right-aligned
/// in 4, in the column where a CW spot carries its WPM (FT8 is 6.25 baud,
/// hence the `6`); the mode; two spaces; `CQ` and the grid the CQ carried,
/// left-aligned in 8, blank for anything but a CQ; the DX's audio offset in
/// Hz right-aligned in 4. That is 28 columns for a three-letter mode, the
/// cell exactly. Modes Aggregator never spots get no rate token, which
/// keeps `offset_from_comment`'s frame intact and the width within the
/// cell for every WSJT-X mode name (the longest, `MSK144`, comes to 27).
fn comment_for(spot: &Spot) -> String {
    if spot.spotter.is_some() {
        return spot.comment.trim().to_string();
    }
    let mut out = format!("{:>3} dB", spot.snr_db);
    match symbol_rate_baud(&spot.mode) {
        Some(baud) => out.push_str(&format!("{baud:>4} {}", spot.mode)),
        None => out.push_str(&format!(" {}", spot.mode)),
    }
    let cq = match (spot.is_cq, &spot.grid) {
        (true, Some(grid)) => format!("CQ {grid}"),
        (true, None) => "CQ".to_string(),
        (false, _) => String::new(),
    };
    out.push_str(&format!("  {cq:<8}"));
    if let Some(hz) = spot.dx_offset_hz() {
        out.push_str(&format!("{hz:>4}"));
    }
    out.trim_end().to_string()
}

/// The modulation rate Aggregator prints in its speed column, whole baud.
/// FT4 is 20.833 baud; `21` is the rounded figure and has not been read off
/// a live Aggregator line (VU2OY's node spotted no FT4 in 2,000 spots) —
/// check it against one when an FT4 spot comes through.
fn symbol_rate_baud(mode: &str) -> Option<u32> {
    match mode.to_ascii_uppercase().as_str() {
        "FT8" => Some(6),
        "FT4" => Some(21),
        _ => None,
    }
}

/// Swift `padding(toLength:)`: pad with spaces to `len` — or truncate.
fn pad_or_trunc(s: &str, len: usize) -> String {
    let mut out: String = s.chars().take(len).collect();
    while out.chars().count() < len {
        out.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spot::Spot;

    /// A spot decoded here: no spotter, no comment, the offset in the field.
    fn decoded(dial_hz: u64, delta_hz: u32, snr: i32, mode: &str, message: &str) -> Spot {
        Spot {
            time_unix: 14 * 3600 + 28 * 60, // 1428Z
            snr_db: snr,
            delta_time_s: 0.0,
            delta_frequency_hz: delta_hz,
            mode: mode.into(),
            mode_inferred: false,
            message: message.into(),
            is_cq: crate::spot::message_is_cq(message),
            comment: String::new(),
            low_confidence: false,
            off_air: false,
            dial_frequency_hz: dial_hz,
            source_name: "MSHV 2333".into(),
            spotter: None,
            is_skimmer: false,
            grid: crate::spot::grid_from_message(message),
            iota: None,
        }
    }

    /// A spot relayed from a cluster node: a spotter, the comment as the
    /// line carried it, the spotted frequency as the dial with no offset.
    fn relayed(comment: &str) -> Spot {
        Spot {
            spotter: Some("W3LPL".into()),
            comment: comment.into(),
            is_cq: true, // synthesised, as nodes.rs does
            grid: None,
            ..decoded(14_075_800, 0, -10, "FT8", "CQ K1JT")
        }
    }

    /// The frame: spotter one token, padded cells, uppercase Z last — the
    /// layout RUMlog tokenises. The comment is Aggregator's, 28 columns.
    #[test]
    fn spider_layout() {
        let line = format(&decoded(28_074_000, 2332, -6, "FT8", "CQ UN7LZ MO13"));
        assert_eq!(
            line,
            "DX de MSHV2333:      28074.0   UN7LZ          -6 dB   6 FT8  CQ MO13 2332 1428Z"
        );
        assert!(line.ends_with('Z'));
        assert_eq!(line.split_whitespace().nth(2), Some("MSHV2333:"));
    }

    /// The three shapes Aggregator gives a decode, column for column:
    /// a CQ with a grid, a CQ without one, a reply (no CQ field at all).
    #[test]
    fn a_decoded_spot_takes_aggregators_shape() {
        let cases = [
            ("CQ UN7LZ MO13", -6, 2332, " -6 dB   6 FT8  CQ MO13 2332"),
            ("CQ UW5KW", -19, 1364, "-19 dB   6 FT8  CQ      1364"),
            ("BG0KE YC2VTS -10", -7, 1758, " -7 dB   6 FT8          1758"),
            ("CQ YC2BUZ", -18, 978, "-18 dB   6 FT8  CQ       978"),
        ];
        for (message, snr, delta, want) in cases {
            let line = format(&decoded(14_074_000, delta, snr, "FT8", message));
            assert!(
                line.contains(&format!(" {want} 1428Z")),
                "want {want:?} in {line:?}"
            );
            assert_eq!(want.len(), 28);
        }
    }

    /// The rate column is the mode's symbol rate: FT4's, not FT8's `6`.
    /// A mode Aggregator never spots gets no rate token at all.
    #[test]
    fn the_rate_column_follows_the_mode() {
        let ft4 = format(&decoded(14_080_000, 1200, -3, "FT4", "CQ K1JT FN20"));
        assert!(ft4.contains(" -3 dB  21 FT4  CQ FN20 1200 "), "got {ft4}");
        let q65 = format(&decoded(50_275_000, 1200, -3, "Q65", "CQ K1JT FN20"));
        assert!(q65.contains(" -3 dB Q65  CQ FN20 1200 "), "got {q65}");
    }

    /// A decode's frequency cell is the dial, not dial + offset: the offset
    /// is in the comment, relative to the dial, and a logger that reads
    /// both must not add it twice.
    #[test]
    fn the_frequency_cell_is_the_dial() {
        let line = format(&decoded(14_074_000, 1758, -7, "FT8", "CQ K1JT FN20"));
        assert!(line.contains(" 14074.0 "), "got {line}");
        assert!(!line.contains("14075.8"), "got {line}");
    }

    /// RUMlog showed the relaying node after `DX de` for every cluster
    /// spot. A relayed spot names its real spotter, a skimmer with `-#`;
    /// a local decode keeps its source name.
    #[test]
    fn de_names_the_real_spotter() {
        let line = format(&relayed("FT8 -10 dB"));
        assert_eq!(line.split_whitespace().nth(2), Some("W3LPL:"), "{line}");

        let skim = Spot {
            spotter: Some("vu2xyz".into()),
            is_skimmer: true,
            ..relayed("")
        };
        let line = format(&skim);
        assert_eq!(line.split_whitespace().nth(2), Some("VU2XYZ-#:"), "{line}");

        let blank = Spot {
            spotter: Some("  ".into()),
            ..relayed("")
        };
        let line = format(&blank);
        assert_eq!(line.split_whitespace().nth(2), Some("MSHV2333:"), "{line}");

        let local = format(&decoded(14_074_000, 1758, -7, "FT8", "CQ K1JT FN20"));
        assert_eq!(local.split_whitespace().nth(2), Some("MSHV2333:"));
    }

    /// A relayed spot's comment goes out verbatim, whatever its shape, and
    /// an empty one stays empty — never a made-up `CQ` or rate.
    #[test]
    fn a_relayed_spots_comment_goes_out_as_it_came() {
        for original in [
            "-11 dB 1794 FT8",
            "-18 dB 6 FT8 CQ KN34 2167",
            "FT8 1500Hz BL11",
            "FT8 -5 dB CQ",
            "12 dB 22 WPM CQ FN20",
        ] {
            let line = format(&relayed(original));
            assert!(
                line.contains(&format!("K1JT          {original} ")),
                "got {line}"
            );
        }
        let empty = format(&relayed(""));
        assert!(
            empty.contains(&format!("K1JT{}1428Z", " ".repeat(10 + 28 + 1))),
            "got {empty}"
        );
        assert!(empty.contains(" 14075.8 "), "got {empty}");
    }

    /// No offset reported (the field at 0, nothing in a comment): the
    /// offset column is simply absent, never `0`.
    #[test]
    fn no_offset_means_no_offset_column() {
        let line = format(&decoded(14_074_000, 0, -10, "FT8", "CQ K1JT FN20"));
        assert!(
            line.contains("K1JT          -10 dB   6 FT8  CQ FN20      1428Z"),
            "got {line}"
        );
    }

    /// What this writes, `offset_from_comment` reads: a DXCA fed by another
    /// DXCA's telnet server sees the offset exactly as it sees VU2OY's.
    #[test]
    fn the_written_offset_reads_back() {
        for (message, delta) in [
            ("CQ UN7LZ MO13", 2332),
            ("CQ UW5KW", 1364),
            ("BG0KE YC2VTS -10", 1758),
        ] {
            let line = format(&decoded(14_074_000, delta, -7, "FT8", message));
            // Tokens: DX de MSHV2333: 14074.0 CALL, then the comment, then the time.
            let toks: Vec<&str> = line.split_whitespace().collect();
            let comment = toks[5..toks.len() - 1].join(" ");
            assert_eq!(
                crate::spot::offset_from_comment(&comment),
                Some(delta),
                "{comment:?}"
            );
        }
    }

    #[test]
    fn no_callsign_becomes_unknown() {
        let s = Spot {
            time_unix: 0,
            snr_db: 3,
            delta_time_s: 0.0,
            delta_frequency_hz: 0,
            mode: "FT4".into(),
            mode_inferred: false,
            message: "73".into(),
            is_cq: true,
            comment: String::new(),
            low_confidence: false,
            off_air: false,
            dial_frequency_hz: 7_047_500,
            source_name: "JTDX".into(),
            spotter: None,
            is_skimmer: false,
            grid: None,
            iota: None,
        };
        assert!(format(&s).contains("UNKNOWN"));
    }
}

#[cfg(test)]
mod source_name_reaches_the_wire {
    use super::*;
    use crate::Spot;

    /// `source_name` IS the spotter callsign on the cluster line, so whatever
    /// is put in it goes out to every connected logger.
    ///
    /// Worth a test because the failure is silent. This function filters the
    /// name to `[A-Z0-9/-]`, so a value carrying punctuation is not rejected
    /// and not truncated — the punctuation is simply dropped, and the result
    /// still looks like a plausible callsign. A namespaced `VU2CPL:MSHV`
    /// becomes `DX de VU2CPLMSHV:`, which nobody would read as a bug.
    #[test]
    fn punctuation_in_a_source_name_is_silently_welded_not_rejected() {
        let mut s = Spot {
            time_unix: 0,
            snr_db: -10,
            delta_time_s: 0.0,
            delta_frequency_hz: 0,
            mode: "FT8".into(),
            mode_inferred: false,
            message: "CQ K1JT".into(),
            is_cq: true,
            comment: String::new(),
            low_confidence: false,
            off_air: false,
            dial_frequency_hz: 14_074_000,
            source_name: "MSHV".into(),
            spotter: None,
            is_skimmer: false,
            grid: None,
            iota: None,
        };
        assert!(format(&s).contains("MSHV:"), "the ordinary case");

        s.source_name = "VU2CPL:MSHV".into();
        let welded = format(&s);
        assert!(welded.contains("VU2CPLMSHV:"), "{welded}");
        assert!(
            !welded.contains("VU2CPL:MSHV"),
            "and nothing here would tell you it happened"
        );
    }
}
