//! Spot classification against a user's log matrix — port of the Swift
//! `AlertClassifier`. Decides New DXCC / New Band / New Mode / New Slot /
//! Worked for one spot and one user; in DXCA 2.0 this runs once per user
//! over the shared spot stream (plan §5).

use crate::beacons;
use crate::dxcc::DxccResolver;
use crate::matrix::LogMatrix;
use crate::{bands, modes};
use serde::{Deserialize, Serialize};

/// Serde names for the first six match the Swift `AlertLevel` raw values, so
/// 1.x-era JSON and the 2.0 API still speak the same strings.
///
/// The `Unconf*` half is DXCA 2.1 and has no 1.x counterpart. It answers a
/// different question from the `New*` half: those mean *never worked*, these
/// mean *worked and still not confirmed* — the QSL/LoTW gap you close by
/// working the same thing again. They coexist deliberately; the old
/// `alert_unconfirmed` switch could only show one or the other, because it
/// swapped the entire comparison over to the confirmed sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "worked")]
    Worked,
    #[serde(rename = "newMode")]
    NewMode,
    #[serde(rename = "newBand")]
    NewBand,
    #[serde(rename = "newSlot")]
    NewSlot,
    #[serde(rename = "newDXCC")]
    NewDxcc,
    #[serde(rename = "unconfMode")]
    UnconfMode,
    #[serde(rename = "unconfBand")]
    UnconfBand,
    #[serde(rename = "unconfSlot")]
    UnconfSlot,
    #[serde(rename = "unconfDXCC")]
    UnconfDxcc,
    // The award axes (docs/AWARDS.md phases 2–4), declared after the
    // original eight so every stored serde value keeps its meaning. Rank is
    // FLAGGABLE's business, not declaration order's.
    #[serde(rename = "newGrid")]
    NewGrid,
    #[serde(rename = "newState")]
    NewState,
    #[serde(rename = "newIOTA")]
    NewIota,
    #[serde(rename = "unconfGrid")]
    UnconfGrid,
    #[serde(rename = "unconfState")]
    UnconfState,
    #[serde(rename = "unconfIOTA")]
    UnconfIota,
    #[serde(rename = "newZone")]
    NewZone,
    #[serde(rename = "unconfZone")]
    UnconfZone,
    /// DX Marathon: an entity or zone not yet worked **this calendar
    /// year**. No `?` twin — the Marathon scores what you worked, so
    /// there is no confirmation gap for it to chase.
    #[serde(rename = "marathon")]
    Marathon,
}

impl AlertLevel {
    /// Every level a spot can be flagged as, rarest first — the one order
    /// the UI, the Telegram labels and the config screens all read from, so
    /// a level cannot rank differently in two places. **This order is also
    /// the tiebreak** when a spot qualifies for several levels at once (a
    /// New DXCC that is also a New Grid flags as New DXCC): the award axes
    /// rank above the generic band/mode rungs because they are named gaps
    /// in a named award, and below the entity itself.
    pub const FLAGGABLE: [AlertLevel; 17] = [
        AlertLevel::NewDxcc,
        AlertLevel::NewIota,
        AlertLevel::NewZone,
        AlertLevel::NewState,
        AlertLevel::NewGrid,
        AlertLevel::Marathon,
        AlertLevel::NewBand,
        AlertLevel::NewMode,
        AlertLevel::NewSlot,
        AlertLevel::UnconfDxcc,
        AlertLevel::UnconfIota,
        AlertLevel::UnconfZone,
        AlertLevel::UnconfState,
        AlertLevel::UnconfGrid,
        AlertLevel::UnconfBand,
        AlertLevel::UnconfMode,
        AlertLevel::UnconfSlot,
    ];

    /// FLAGGABLE rank, for picking among simultaneous candidates. Worked
    /// and None rank below everything flaggable.
    fn rank(self) -> usize {
        AlertLevel::FLAGGABLE
            .iter()
            .position(|l| *l == self)
            .unwrap_or(usize::MAX)
    }

    /// The confirmation-hunting half of the ladder — the `?` levels.
    /// The `docs/AWARDS.md` phase-1 gate applies to exactly these, the
    /// award `?` levels included: a `? Grid` ping for a non-QSLer is the
    /// same wasted call a `? DXCC` ping is.
    pub fn is_unconfirmed(self) -> bool {
        matches!(
            self,
            AlertLevel::UnconfDxcc
                | AlertLevel::UnconfBand
                | AlertLevel::UnconfMode
                | AlertLevel::UnconfSlot
                | AlertLevel::UnconfGrid
                | AlertLevel::UnconfState
                | AlertLevel::UnconfIota
                | AlertLevel::UnconfZone
        )
    }

    /// Which chased award a level belongs to — `None` for the classic
    /// DXCC eight. Served with the reference vocabulary so the UI can keep
    /// an unchased award's levels out of every ladder, chip row and
    /// badge: opting out of an award means never seeing its controls.
    pub fn award(self) -> Option<&'static str> {
        match self {
            AlertLevel::NewGrid | AlertLevel::UnconfGrid => Some("vucc"),
            AlertLevel::NewState | AlertLevel::UnconfState => Some("was"),
            AlertLevel::NewIota | AlertLevel::UnconfIota => Some("iota"),
            AlertLevel::NewZone | AlertLevel::UnconfZone => Some("waz"),
            AlertLevel::Marathon => Some("marathon"),
            _ => None,
        }
    }

    /// Stable machine key — the same string serde emits.
    pub fn key(self) -> &'static str {
        match self {
            AlertLevel::None => "none",
            AlertLevel::Worked => "worked",
            AlertLevel::NewMode => "newMode",
            AlertLevel::NewBand => "newBand",
            AlertLevel::NewSlot => "newSlot",
            AlertLevel::NewDxcc => "newDXCC",
            AlertLevel::UnconfMode => "unconfMode",
            AlertLevel::UnconfBand => "unconfBand",
            AlertLevel::UnconfSlot => "unconfSlot",
            AlertLevel::UnconfDxcc => "unconfDXCC",
            AlertLevel::NewGrid => "newGrid",
            AlertLevel::NewState => "newState",
            AlertLevel::NewIota => "newIOTA",
            AlertLevel::UnconfGrid => "unconfGrid",
            AlertLevel::UnconfState => "unconfState",
            AlertLevel::UnconfIota => "unconfIOTA",
            AlertLevel::NewZone => "newZone",
            AlertLevel::UnconfZone => "unconfZone",
            AlertLevel::Marathon => "marathon",
        }
    }

    /// Short human label. `?` is the logger convention for worked-but-unconfirmed.
    pub fn label(self) -> &'static str {
        match self {
            AlertLevel::None => "",
            AlertLevel::Worked => "",
            AlertLevel::NewMode => "New Mode",
            AlertLevel::NewBand => "New Band",
            AlertLevel::NewSlot => "New Slot",
            AlertLevel::NewDxcc => "NEW DXCC",
            AlertLevel::UnconfMode => "? Mode",
            AlertLevel::UnconfBand => "? Band",
            AlertLevel::UnconfSlot => "? Slot",
            AlertLevel::UnconfDxcc => "? DXCC",
            AlertLevel::NewGrid => "New Grid",
            AlertLevel::NewState => "New State",
            AlertLevel::NewIota => "New IOTA",
            AlertLevel::UnconfGrid => "? Grid",
            AlertLevel::UnconfState => "? State",
            AlertLevel::UnconfIota => "? IOTA",
            AlertLevel::NewZone => "New Zone",
            AlertLevel::UnconfZone => "? Zone",
            AlertLevel::Marathon => "Marathon",
        }
    }
}

/// **What counts as a new state** — the WAS award you are actually chasing.
///
/// The same New/? levels serve all three; what changes is the question they
/// ask, so picking a scope adds no rows to any ladder or chip group.
///
/// This is a *chasing* setting, not a reporting one. An operator after
/// Triple Play needs to be told about Ohio on phone even though Ohio is
/// long since worked on CW — under `Mixed` that spot is silent, which is
/// correct for basic WAS and useless for the award they are chasing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WasScope {
    /// Basic WAS: the state on any band, in any mode.
    #[serde(rename = "mixed")]
    #[default]
    Mixed,
    /// **Triple Play**: the state in each of CW, Phone and Digital.
    #[serde(rename = "triple")]
    TriplePlay,
    /// Band endorsements: the state on each band.
    #[serde(rename = "band")]
    PerBand,
}

impl WasScope {
    pub fn key(self) -> &'static str {
        match self {
            WasScope::Mixed => "mixed",
            WasScope::TriplePlay => "triple",
            WasScope::PerBand => "band",
        }
    }
}

/// The per-user alert toggles the classifier consults. A level switched off
/// here is never flagged at all — it collapses to `Worked`, exactly as the
/// four 1.x toggles always did.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub alert_new_dxcc: bool,
    pub alert_new_slot: bool,
    pub alert_new_band: bool,
    pub alert_new_mode: bool,
    // DXCA 2.1: the confirmation-hunting half. Off by default — a fresh
    // install behaves exactly like 1.x until the operator asks for these.
    pub alert_unconf_dxcc: bool,
    pub alert_unconf_slot: bool,
    pub alert_unconf_band: bool,
    pub alert_unconf_mode: bool,
    // docs/AWARDS.md phases 2–4: the award axes, all off by default. A pair
    // ticked here IS the award selector — there is no separate "enable
    // VUCC" switch, because a level that can never be flagged and an award
    // that is off are the same fact.
    pub alert_new_grid: bool,
    pub alert_unconf_grid: bool,
    pub alert_new_state: bool,
    pub alert_unconf_state: bool,
    /// Which WAS the State levels are hunting — see [`WasScope`].
    #[serde(default)]
    pub was_scope: WasScope,
    pub alert_new_iota: bool,
    pub alert_unconf_iota: bool,
    pub alert_new_zone: bool,
    pub alert_unconf_zone: bool,
    /// Which WAZ the Zone levels hunt: mixed, or per band (5-band WAZ).
    /// The mode scope WAS has does not apply — WAZ has no mode award.
    #[serde(default)]
    pub waz_scope: WazScope,
    /// The DX Marathon: an entity or zone not yet worked this year.
    pub alert_marathon: bool,
}

/// **What counts as a new zone** — the WAZ variant being chased.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WazScope {
    /// Basic WAZ: the zone on any band.
    #[serde(rename = "mixed")]
    #[default]
    Mixed,
    /// Five-band WAZ and its kin: the zone on each band.
    #[serde(rename = "band")]
    PerBand,
    /// The zone in each mode class — the WAZ counterpart of Triple Play.
    #[serde(rename = "mode")]
    PerMode,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            alert_new_dxcc: true,
            alert_new_slot: true,
            alert_new_band: true,
            alert_new_mode: true,
            alert_unconf_dxcc: false,
            alert_unconf_slot: false,
            alert_unconf_band: false,
            alert_unconf_mode: false,
            alert_new_grid: false,
            alert_unconf_grid: false,
            alert_new_state: false,
            alert_unconf_state: false,
            was_scope: WasScope::Mixed,
            alert_new_iota: false,
            alert_unconf_iota: false,
            alert_new_zone: false,
            alert_unconf_zone: false,
            waz_scope: WazScope::Mixed,
            alert_marathon: false,
        }
    }
}

/// The spot-side award facts a classifier can rank — what the wire carried
/// (grid, IOTA ref) and what the server looked up (state). All optional;
/// [`AwardRefs::NONE`] classifies exactly as before the awards existed.
#[derive(Debug, Clone, Copy, Default)]
pub struct AwardRefs<'a> {
    /// The DX station's locator, 4 or 6 characters.
    pub grid: Option<&'a str>,
    /// IOTA reference as extracted (normalized again internally).
    pub iota: Option<&'a str>,
    /// Two-letter state from the FCC table, already normalized.
    pub state: Option<&'a str>,
    /// The CQ zone the spot resolves to — WAZ and the Marathon.
    pub zone: Option<i32>,
    /// The DXCC entity, for the Marathon's entity half. Supplied even
    /// though `classify_spot` resolves one itself, because the Marathon
    /// needs it beside the zone and the year in one place.
    pub year: Option<i32>,
}

impl AwardRefs<'static> {
    pub const NONE: AwardRefs<'static> = AwardRefs {
        grid: None,
        iota: None,
        state: None,
        zone: None,
        year: None,
    };
}

#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    pub level: AlertLevel,
    pub dxcc_id: Option<i32>,
    pub dxcc_name: Option<String>,
    pub band: Option<&'static str>,
    pub is_beacon: bool,
    /// The award key that fired, when `level` is an award level — the grid
    /// square, state, or IOTA reference. What the alert names as the catch.
    pub award_ref: Option<String>,
}

impl Classification {
    fn none() -> Self {
        Classification {
            level: AlertLevel::None,
            dxcc_id: None,
            dxcc_name: None,
            band: None,
            is_beacon: false,
            award_ref: None,
        }
    }
}

pub struct AlertClassifier<'a> {
    pub matrix: &'a LogMatrix,
    pub resolver: &'a DxccResolver,
    pub config: &'a AlertConfig,
}

impl AlertClassifier<'_> {
    /// Classify a spot with no award facts — the pre-phase-2 behaviour,
    /// kept for callers (and tests) that have nothing but call/freq/mode.
    pub fn classify(&self, callsign: &str, frequency_mhz: f64, mode: &str) -> Classification {
        self.classify_spot(callsign, frequency_mhz, mode, &AwardRefs::NONE)
    }

    /// Classify a spot. `None`-level when data is missing (no matrix entry
    /// needed — that's New DXCC — but no resolver, band, or DXCC id).
    ///
    /// A spot can qualify for several levels at once — a station in a new
    /// grid can be a New DXCC too. One spot carries one level, so the
    /// candidates are ranked by [`AlertLevel::FLAGGABLE`] and the rarest
    /// wins; a level the operator switched off simply never becomes a
    /// candidate, so switching NEW DXCC off makes the same spot flag as
    /// its next-rarest truth rather than vanish.
    pub fn classify_spot(
        &self,
        callsign: &str,
        frequency_mhz: f64,
        mode: &str,
        refs: &AwardRefs,
    ) -> Classification {
        if callsign.is_empty() || !self.resolver.is_loaded() {
            return Classification::none();
        }

        let band = bands::band_from_mhz(frequency_mhz);

        // Beacons / satellites / gateways (ClubLog adif=0, or our own
        // beacon database): label them, never alert — no award either.
        let known_beacon = beacons::display_name(callsign);
        if self.resolver.is_non_dx_operation(callsign) || known_beacon.is_some() {
            return Classification {
                level: AlertLevel::None,
                dxcc_id: None,
                dxcc_name: Some(known_beacon.unwrap_or_else(|| "Beacon".to_string())),
                band,
                is_beacon: true,
                award_ref: None,
            };
        }

        let dxcc_id = self.resolver.resolve(callsign);
        let dxcc_name = dxcc_id
            .and_then(|id| self.resolver.entity(id))
            .map(|e| e.name.clone());

        // WSPR / FST4W are one-way beacon modes: the station cannot be
        // worked, so no level and no award. The entity still resolves so
        // the spot list can name it.
        if modes::is_beacon_mode(mode) {
            return Classification {
                level: AlertLevel::None,
                dxcc_id,
                dxcc_name,
                band,
                is_beacon: false,
                award_ref: None,
            };
        }

        // FT8/FT4/JT*/RTTY/… → DATA: digital modes share one award slot.
        // `None` when the mode is genuinely unknown — `canonical` would call
        // that DATA, which is how phone spots from comment-only cluster
        // nodes used to be credited to digital slots.
        let normalized_mode = modes::canonical_opt(mode);

        // Award candidates need only a band, not an entity — a call the
        // resolver cannot place can still hand you a grid square.
        let award = band.and_then(|b| self.best_award(refs, b, normalized_mode, dxcc_id));

        let (Some(dxcc), Some(bnd)) = (dxcc_id, band) else {
            let (level, award_ref) = match award {
                Some((l, r)) => (l, Some(r)),
                None => (AlertLevel::None, None),
            };
            return Classification {
                level,
                dxcc_id,
                dxcc_name,
                band,
                is_beacon: false,
                award_ref,
            };
        };

        let dxcc_level = self.apply_filter(self.raw_level(dxcc, bnd, normalized_mode));
        let (level, award_ref) = match award {
            Some((l, r)) if l.rank() < dxcc_level.rank() => (l, Some(r)),
            _ => (dxcc_level, None),
        };
        Classification {
            level,
            dxcc_id: Some(dxcc),
            dxcc_name,
            band: Some(bnd),
            is_beacon: false,
            award_ref,
        }
    }

    /// The rarest enabled award gap this spot fills, with the key that
    /// fills it. Grid is per band (VUCC scores each band separately, and
    /// only 50 MHz+); state and IOTA are key-level — worked at all, then
    /// confirmed on any band.
    fn best_award(
        &self,
        refs: &AwardRefs,
        band: &str,
        mode: Option<&str>,
        dxcc_id: Option<i32>,
    ) -> Option<(AlertLevel, String)> {
        let mut cands: Vec<(AlertLevel, String)> = Vec::new();

        if bands::is_vucc_band(band)
            && let Some(g4) = refs.grid.and_then(crate::grid::grid4)
        {
            let raw = match self.matrix.by_grid.get(&g4) {
                Some(s) if s.confirmed_bands.contains(band) => AlertLevel::Worked,
                Some(s) if s.bands.contains(band) => AlertLevel::UnconfGrid,
                _ => AlertLevel::NewGrid,
            };
            cands.push((raw, g4));
        }
        // A state means nothing unless the station is IN the US: a call
        // like `DV2/K7AZQ` is an Arizona licensee transmitting from the
        // Philippines, and its licence address is not a WAS credit. The
        // entity the resolver already worked out is the reliable test —
        // `StateTable::lookup` refuses the same call independently, and
        // both guards earn their place (2026-09-01).
        // An EMPTY axis means "no data", not "nothing worked", for these
        // two: their only source is the optional LoTW QSL report, which can
        // be absent, refused, or — as it was until 2.17.5 — silently
        // incremental. With nothing in the map every state on the air looks
        // new, which is the loudest possible way to be wrong.
        //
        // `by_grid` is deliberately NOT guarded this way: it comes from the
        // same ClubLog log that drives DXCC, so empty there really does mean
        // no 50 MHz+ grids worked, and a first New Grid is then correct.
        if let Some(st) = refs
            .state
            .filter(|_| !self.matrix.by_state.is_empty())
            .filter(|_| dxcc_id.is_some_and(crate::awards::counts_for_was))
            .and_then(crate::awards::normalize_state)
        {
            let have = self.matrix.by_state.get(st);
            // The scope decides what "new" means. Triple Play and the band
            // endorsements need the state ON THIS SLICE, so a state long
            // since worked is still a catch in a mode or on a band it is
            // missing from — which is the entire reason to chase them.
            let raw = match self.config.was_scope {
                WasScope::Mixed => match have {
                    Some(s) if s.is_confirmed() => AlertLevel::Worked,
                    Some(_) => AlertLevel::UnconfState,
                    None => AlertLevel::NewState,
                },
                // No mode means no honest answer for Triple Play — a spot
                // whose mode we had to guess must not claim a slot in an
                // award counted in modes. Same rule the DXCC ladder follows.
                WasScope::TriplePlay => match mode {
                    None => AlertLevel::Worked,
                    Some(m) => match have {
                        Some(s) if s.confirmed_modes.contains(m) => AlertLevel::Worked,
                        Some(s) if s.modes.contains(m) => AlertLevel::UnconfState,
                        _ => AlertLevel::NewState,
                    },
                },
                WasScope::PerBand => match have {
                    Some(s) if s.confirmed_bands.contains(band) => AlertLevel::Worked,
                    Some(s) if s.bands.contains(band) => AlertLevel::UnconfState,
                    _ => AlertLevel::NewState,
                },
            };
            // The key names the slice, so the alert says WHY: "OH CW" for a
            // Triple Play gap reads very differently from a bare "OH".
            let key = match (self.config.was_scope, mode) {
                (WasScope::TriplePlay, Some(m)) => format!("{st} {m}"),
                (WasScope::PerBand, _) => format!("{st} {band}"),
                _ => st.to_string(),
            };
            cands.push((raw, key));
        }
        // WAZ. The zone axis rides ClubLog's own `CQZ`, so unlike states
        // and islands it is populated for anyone with a log — no
        // empty-axis guard is needed or wanted here.
        if let Some(z) = refs.zone.filter(|z| (1..=40).contains(z)) {
            let have = self.matrix.by_zone.get(&z);
            let raw = match self.config.waz_scope {
                WazScope::Mixed => match have {
                    Some(s) if s.is_confirmed() => AlertLevel::Worked,
                    Some(_) => AlertLevel::UnconfZone,
                    None => AlertLevel::NewZone,
                },
                WazScope::PerBand => match have {
                    Some(s) if s.confirmed_bands.contains(band) => AlertLevel::Worked,
                    Some(s) if s.bands.contains(band) => AlertLevel::UnconfZone,
                    _ => AlertLevel::NewZone,
                },
                // No mode, no honest answer — the same rule Triple Play
                // follows, and for the same reason: a guessed mode must not
                // claim a slot in an award counted in modes.
                WazScope::PerMode => match mode {
                    None => AlertLevel::Worked,
                    Some(m) => match have {
                        Some(s) if s.confirmed_modes.contains(m) => AlertLevel::Worked,
                        Some(s) if s.modes.contains(m) => AlertLevel::UnconfZone,
                        _ => AlertLevel::NewZone,
                    },
                },
            };
            let key = match (self.config.waz_scope, mode) {
                (WazScope::PerBand, _) => format!("Zone {z} {band}"),
                (WazScope::PerMode, Some(m)) => format!("Zone {z} {m}"),
                _ => format!("Zone {z}"),
            };
            cands.push((raw, key));
        }

        // DX Marathon: is this entity or zone missing from THIS year? A
        // lifetime-worked entity is still a Marathon point every January,
        // which is exactly what makes the award worth its own level.
        if let Some(year) = refs.year {
            let this_year = self.matrix.by_year.get(&year);
            let entity_new =
                dxcc_id.is_some_and(|d| !this_year.is_some_and(|y| y.entities.contains(&d)));
            let zone_new = refs
                .zone
                .filter(|z| (1..=40).contains(z))
                .is_some_and(|z| !this_year.is_some_and(|y| y.zones.contains(&z)));
            if entity_new || zone_new {
                // Name the half that is actually missing — "Zone 22" and
                // "INDIA" send you to the radio for different reasons.
                let key = match (entity_new, zone_new, refs.zone) {
                    (true, true, Some(z)) => format!("entity + zone {z}"),
                    (false, true, Some(z)) => format!("zone {z}"),
                    _ => "entity".to_string(),
                };
                cands.push((AlertLevel::Marathon, key));
            }
        }

        if let Some(r) = refs
            .iota
            .filter(|_| !self.matrix.by_iota.is_empty())
            .and_then(crate::awards::normalize_iota)
        {
            let raw = match self.matrix.by_iota.get(&r) {
                Some(s) if s.is_confirmed() => AlertLevel::Worked,
                Some(_) => AlertLevel::UnconfIota,
                None => AlertLevel::NewIota,
            };
            cands.push((raw, r));
        }

        cands
            .into_iter()
            .map(|(raw, key)| (self.apply_filter(raw), key))
            .filter(|(l, _)| !matches!(l, AlertLevel::Worked | AlertLevel::None))
            .min_by_key(|(l, _)| l.rank())
    }

    /// The ladder, rarest gap first. Two passes over the same entity: the
    /// WORKED sets decide the `New*` half, and only once a spot survives all
    /// of those (i.e. the slot really is in the log) do the CONFIRMED sets
    /// decide the `Unconf*` half. That ordering is the whole point — a band
    /// you have never worked is a better catch than one you have worked and
    /// not confirmed, so it must win even though both are "missing".
    fn raw_level(&self, dxcc: i32, band: &str, mode: Option<&str>) -> AlertLevel {
        let Some(status) = self.matrix.status(dxcc) else {
            return AlertLevel::NewDxcc;
        };

        // No mode means no slot and no mode gap that can honestly be
        // answered, so only the band half of the ladder runs. Inventing a
        // slot would put the spot in an award bucket it may not belong to;
        // returning nothing would hide a genuinely new band. Band gaps are
        // mode-independent, so they stay.
        let Some(mode) = mode else {
            if !status.bands.contains(band) {
                return AlertLevel::NewBand;
            }
            if status.confirmed_slots.is_empty() {
                return AlertLevel::UnconfDxcc;
            }
            if !status.confirmed_bands.contains(band) {
                return AlertLevel::UnconfBand;
            }
            return AlertLevel::Worked;
        };

        let slot = format!("{band}-{mode}");

        // --- never worked -------------------------------------------------
        // Priority: never-worked band > never-worked mode > new combination
        // of both (the genuine 5BDXCC-style slot).
        if !status.slots.contains(&slot) {
            if !status.bands.contains(band) {
                return AlertLevel::NewBand;
            }
            if !status.modes.contains(mode) {
                return AlertLevel::NewMode;
            }
            return AlertLevel::NewSlot;
        }

        // --- worked, but is it confirmed? ---------------------------------
        // Same shape one level down. The entity-wide check comes first: with
        // nothing at all confirmed the band/mode/slot gaps are all true too,
        // and "this entity is entirely unconfirmed" is the bigger fact.
        if status.confirmed_slots.is_empty() {
            return AlertLevel::UnconfDxcc;
        }
        if !status.confirmed_slots.contains(&slot) {
            if !status.confirmed_bands.contains(band) {
                return AlertLevel::UnconfBand;
            }
            if !status.confirmed_modes.contains(mode) {
                return AlertLevel::UnconfMode;
            }
            return AlertLevel::UnconfSlot;
        }
        AlertLevel::Worked
    }

    /// A level the operator switched off is not flagged at all. Note this
    /// does NOT fall through to the next-best level: switching off New Band
    /// means "don't tell me about bands", not "tell me it's a slot instead".
    fn apply_filter(&self, level: AlertLevel) -> AlertLevel {
        let keep = match level {
            AlertLevel::NewDxcc => self.config.alert_new_dxcc,
            AlertLevel::NewSlot => self.config.alert_new_slot,
            AlertLevel::NewBand => self.config.alert_new_band,
            AlertLevel::NewMode => self.config.alert_new_mode,
            AlertLevel::UnconfDxcc => self.config.alert_unconf_dxcc,
            AlertLevel::UnconfSlot => self.config.alert_unconf_slot,
            AlertLevel::UnconfBand => self.config.alert_unconf_band,
            AlertLevel::UnconfMode => self.config.alert_unconf_mode,
            AlertLevel::NewGrid => self.config.alert_new_grid,
            AlertLevel::UnconfGrid => self.config.alert_unconf_grid,
            AlertLevel::NewState => self.config.alert_new_state,
            AlertLevel::UnconfState => self.config.alert_unconf_state,
            AlertLevel::NewIota => self.config.alert_new_iota,
            AlertLevel::UnconfIota => self.config.alert_unconf_iota,
            AlertLevel::NewZone => self.config.alert_new_zone,
            AlertLevel::UnconfZone => self.config.alert_unconf_zone,
            AlertLevel::Marathon => self.config.alert_marathon,
            AlertLevel::Worked | AlertLevel::None => return level,
        };
        if keep { level } else { AlertLevel::Worked }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cty::{DxccEntity, PrefixRule};
    use std::collections::HashMap;

    fn resolver() -> DxccResolver {
        let mut entities = HashMap::new();
        entities.insert(
            324,
            DxccEntity {
                adif: 324,
                name: "INDIA".into(),
                prefix: "VU".into(),
                cq_zone: 22,
                continent: "AS".into(),
                deleted: false,
                ..Default::default()
            },
        );
        entities.insert(
            291,
            DxccEntity {
                adif: 291,
                name: "UNITED STATES".into(),
                prefix: "K".into(),
                cq_zone: 5,
                continent: "NA".into(),
                deleted: false,
                ..Default::default()
            },
        );
        let rules = vec![
            PrefixRule {
                call: "VU".into(),
                adif: 324,
                is_exact: false,
                start_unix: None,
                end_unix: None,
                cq_zone: None,
            },
            PrefixRule {
                call: "K".into(),
                adif: 291,
                is_exact: false,
                start_unix: None,
                end_unix: None,
                cq_zone: None,
            },
        ];
        let mut r = DxccResolver::default();
        r.load(
            crate::cty::CtyData {
                entities,
                prefix_rules: rules,
                ..Default::default()
            },
            0,
        );
        r
    }

    fn matrix() -> LogMatrix {
        let mut m = LogMatrix::default();
        // India worked: 20M DATA (confirmed), 40M CW (unconfirmed).
        m.record(324, "20M", "DATA", "VU2AAA", true);
        m.record(324, "40M", "CW", "VU2BBB", false);
        m
    }

    fn classify(call: &str, mhz: f64, mode: &str, config: &AlertConfig) -> Classification {
        let m = matrix();
        let r = resolver();
        AlertClassifier {
            matrix: &m,
            resolver: &r,
            config,
        }
        .classify(call, mhz, mode)
    }

    #[test]
    fn unconfirmed_is_exactly_the_question_half() {
        for level in AlertLevel::FLAGGABLE {
            assert_eq!(level.is_unconfirmed(), level.key().starts_with("unconf"));
        }
        assert!(!AlertLevel::Worked.is_unconfirmed());
        assert!(!AlertLevel::None.is_unconfirmed());
    }

    /// All awards on, all levels on — the widest config for award tests.
    fn awards_config() -> AlertConfig {
        AlertConfig {
            alert_unconf_dxcc: true,
            alert_unconf_slot: true,
            alert_unconf_band: true,
            alert_unconf_mode: true,
            alert_new_grid: true,
            alert_unconf_grid: true,
            alert_new_state: true,
            alert_unconf_state: true,
            alert_new_iota: true,
            alert_unconf_iota: true,
            ..AlertConfig::default()
        }
    }

    fn classify_refs(
        m: &LogMatrix,
        call: &str,
        mhz: f64,
        config: &AlertConfig,
        refs: AwardRefs,
    ) -> Classification {
        let r = resolver();
        AlertClassifier {
            matrix: m,
            resolver: &r,
            config,
        }
        .classify_spot(call, mhz, "FT8", &refs)
    }

    #[test]
    fn award_ladder_grid_is_per_band_and_vucc_only() {
        let cfg = awards_config();
        let mut m = matrix();
        // India worked+confirmed on 6M-DATA too, so the DXCC ladder is
        // satisfied there and the grid axis has the floor. (Without this
        // the 6M spots flag New Band — rank says an unworked band beats a
        // grid gap, which is its own test below.)
        m.record(324, "6M", "DATA", "VU2CCC", true);
        m.record_grid("MK83", "6M", Some("DATA"), false);

        let grid = |g| AwardRefs {
            grid: Some(g),
            ..AwardRefs::NONE
        };
        // 6M, unknown square → New Grid, and the ref names the square.
        let c = classify_refs(&m, "VU2XYZ", 50.313, &cfg, grid("MK97FK"));
        assert_eq!(c.level, AlertLevel::NewGrid);
        assert_eq!(c.award_ref.as_deref(), Some("MK97"));
        // Same square, worked on 6M but unconfirmed → ? Grid.
        let c = classify_refs(&m, "VU2XYZ", 50.313, &cfg, grid("MK83VA"));
        assert_eq!(c.level, AlertLevel::UnconfGrid);
        // Worked square, DIFFERENT VUCC band → New Grid again (per band).
        let c = classify_refs(&m, "VU2XYZ", 144.174, &cfg, grid("MK83"));
        assert_eq!(c.level, AlertLevel::NewGrid);
        // The same grid on 20M scores nothing: VUCC is 50 MHz+.
        let c = classify_refs(&m, "VU2XYZ", 14.074, &cfg, grid("MK97"));
        assert_ne!(c.level, AlertLevel::NewGrid);
        // RR73 is a sign-off, not a square.
        let c = classify_refs(&m, "VU2XYZ", 50.313, &cfg, grid("RR73"));
        assert_ne!(c.level, AlertLevel::NewGrid);
    }

    #[test]
    fn award_ladder_state_and_iota_are_key_level() {
        let cfg = awards_config();
        let mut m = matrix();
        // The US worked+confirmed on 20M-DATA, so a K call on 20M FT8
        // leaves the DXCC ladder quiet and the state axis decides.
        m.record(291, "20M", "DATA", "K9XX", true);
        m.record_state("OH", "20M", Some("DATA"), false);
        m.record_state("CA", "20M", Some("DATA"), true);
        m.record_iota("AS-003", "15M", Some("DATA"), false);

        let c = classify_refs(
            &m,
            "K1ABC",
            14.074,
            &cfg,
            AwardRefs {
                state: Some("TX"),
                ..AwardRefs::NONE
            },
        );
        assert_eq!(c.level, AlertLevel::NewState);
        assert_eq!(c.award_ref.as_deref(), Some("TX"));
        let unconf = classify_refs(
            &m,
            "K1ABC",
            14.074,
            &cfg,
            AwardRefs {
                state: Some("OH"),
                ..AwardRefs::NONE
            },
        );
        assert_eq!(unconf.level, AlertLevel::UnconfState);
        let done = classify_refs(
            &m,
            "K1ABC",
            14.074,
            &cfg,
            AwardRefs {
                state: Some("CA"),
                ..AwardRefs::NONE
            },
        );
        assert_ne!(done.level, AlertLevel::NewState);
        assert_ne!(done.level, AlertLevel::UnconfState);

        let iota = classify_refs(
            &m,
            "VU2ABC",
            14.074,
            &cfg,
            AwardRefs {
                iota: Some("as-153"),
                ..AwardRefs::NONE
            },
        );
        assert_eq!(iota.level, AlertLevel::NewIota);
        assert_eq!(iota.award_ref.as_deref(), Some("AS-153"));
        let unconf_iota = classify_refs(
            &m,
            "VU2ABC",
            14.074,
            &cfg,
            AwardRefs {
                iota: Some("AS-003"),
                ..AwardRefs::NONE
            },
        );
        assert_eq!(unconf_iota.level, AlertLevel::UnconfIota);
    }

    /// The reported bug: `DV2/K7AZQ` is a US licensee operating from the
    /// Philippines. The FCC table knows the call, so a state ref can still
    /// arrive — the entity is what has to refuse it.
    /// With no state data at all — no LoTW report, or one that came back
    /// empty — every state on the air would flag as new. Silence is the
    /// honest answer; the award starts working the moment the log does.
    /// WAZ counts the zone, and the Marathon counts the same contact again
    /// for the calendar year — the two awards this pair of levels serves.
    #[test]
    fn waz_counts_zones_and_the_marathon_counts_this_year() {
        let mut cfg = awards_config();
        cfg.alert_new_zone = true;
        cfg.alert_unconf_zone = true;
        cfg.alert_marathon = true;

        let mut m = matrix();
        // Zone 22 confirmed on 20M; nothing at all in zone 25.
        m.record_zone(22, "20M", Some("DATA"), true);
        // India worked this year, but the zone was not.
        m.record_marathon(2026, Some(324), None);

        let at = |cfg: &AlertConfig, call: &str, zone: i32, year: Option<i32>| {
            let r = resolver();
            AlertClassifier {
                matrix: &m,
                resolver: &r,
                config: cfg,
            }
            .classify_spot(
                call,
                14.074,
                "FT8",
                &AwardRefs {
                    zone: Some(zone),
                    year,
                    ..AwardRefs::NONE
                },
            )
        };

        // A zone never worked is a New Zone.
        assert_eq!(at(&cfg, "JA1ABC", 25, None).level, AlertLevel::NewZone);
        // Zone 22 is confirmed, so WAZ is quiet — but the MARATHON still
        // wants it, because zone 22 is missing from this year.
        let c = at(&cfg, "VU2XYZ", 22, Some(2026));
        assert_eq!(c.level, AlertLevel::Marathon);
        assert_eq!(c.award_ref.as_deref(), Some("zone 22"));
        // Same spot with the Marathon switched off falls silent.
        let mut no_marathon = cfg.clone();
        no_marathon.alert_marathon = false;
        assert_ne!(
            at(&no_marathon, "VU2XYZ", 22, Some(2026)).level,
            AlertLevel::Marathon
        );
        // A different YEAR has nothing recorded, so the entity counts too.
        assert_eq!(
            at(&cfg, "VU2XYZ", 22, Some(2027)).award_ref.as_deref(),
            Some("entity + zone 22")
        );

        // Per-band WAZ: zone 22 is held on 20M and missing on 40M.
        let mut per_band = cfg.clone();
        per_band.waz_scope = WazScope::PerBand;
        per_band.alert_marathon = false;
        let r = resolver();
        let on_40 = AlertClassifier {
            matrix: &m,
            resolver: &r,
            config: &per_band,
        }
        .classify_spot(
            "VU2XYZ",
            7.074,
            "FT8",
            &AwardRefs {
                zone: Some(22),
                ..AwardRefs::NONE
            },
        );
        assert_eq!(on_40.level, AlertLevel::NewZone, "40M is missing");
        assert_eq!(on_40.award_ref.as_deref(), Some("Zone 22 40M"));
    }

    /// The scope is a CHASING setting: it changes what counts as new, so a
    /// state long since worked is still a catch in a mode or on a band it
    /// is missing from. That is the whole reason to pick one.
    #[test]
    fn the_was_scope_decides_what_a_new_state_means() {
        let mut m = matrix();
        m.record(291, "20M", "DATA", "K9XX", true);
        // Ohio: confirmed on 20M in DATA, and nowhere else.
        m.record_state("OH", "20M", Some("DATA"), true);

        let refs = AwardRefs {
            state: Some("OH"),
            ..AwardRefs::NONE
        };
        // The mode is passed explicitly: `classify_refs` hardcodes FT8, and
        // the whole point here is that the mode decides the answer.
        let at = |cfg: &AlertConfig, mhz: f64, mode: &str| {
            let r = resolver();
            AlertClassifier {
                matrix: &m,
                resolver: &r,
                config: cfg,
            }
            .classify_spot("K1ABC", mhz, mode, &refs)
            .level
        };

        // Mixed: worked is worked, whatever the band or mode.
        let mut mixed = awards_config();
        mixed.was_scope = WasScope::Mixed;
        assert_ne!(at(&mixed, 14.074, "FT8"), AlertLevel::NewState);
        assert_ne!(at(&mixed, 7.020, "CW"), AlertLevel::NewState, "still done");

        // Triple Play: the same state in a mode it lacks IS new.
        let mut triple = awards_config();
        triple.was_scope = WasScope::TriplePlay;
        assert_ne!(
            at(&triple, 14.074, "FT8"),
            AlertLevel::NewState,
            "DATA held"
        );
        assert_eq!(
            at(&triple, 14.020, "CW"),
            AlertLevel::NewState,
            "CW missing"
        );
        assert_eq!(
            at(&triple, 14.200, "SSB"),
            AlertLevel::NewState,
            "phone missing"
        );

        // Per band: the same state on a band it lacks is new.
        let mut band = awards_config();
        band.was_scope = WasScope::PerBand;
        assert_ne!(at(&band, 14.074, "FT8"), AlertLevel::NewState, "20M held");
        assert_eq!(at(&band, 7.074, "FT8"), AlertLevel::NewState, "40M missing");
    }

    #[test]
    fn an_empty_axis_claims_nothing_is_new() {
        let cfg = awards_config();
        let mut m = matrix();
        m.record(291, "20M", "DATA", "K9XX", true);

        let refs = AwardRefs {
            state: Some("TX"),
            iota: Some("AS-153"),
            ..AwardRefs::NONE
        };
        let quiet = classify_refs(&m, "K1ABC", 14.074, &cfg, refs);
        assert_ne!(quiet.level, AlertLevel::NewState, "no state data, no claim");
        assert_ne!(quiet.level, AlertLevel::NewIota, "no island data, no claim");

        // One known state is enough to make the axis trustworthy again.
        m.record_state("OH", "20M", Some("DATA"), true);
        let live = classify_refs(
            &m,
            "K1ABC",
            14.074,
            &cfg,
            AwardRefs {
                state: Some("TX"),
                ..AwardRefs::NONE
            },
        );
        assert_eq!(live.level, AlertLevel::NewState);
    }

    #[test]
    fn a_us_call_operating_abroad_is_not_a_new_state() {
        let cfg = awards_config();
        let mut m = matrix();
        // A worked state, so the empty-axis guard is not what makes this
        // pass — the DXCC test is what is under examination here.
        m.record_state("OH", "20M", Some("DATA"), true);
        // India stands in for "somewhere that is not the US" — the resolver
        // in these tests knows VU and K, and VU is not a WAS entity.
        let abroad = classify_refs(
            &m,
            "VU2XYZ",
            14.074,
            &cfg,
            AwardRefs {
                state: Some("AZ"),
                ..AwardRefs::NONE
            },
        );
        assert_ne!(abroad.level, AlertLevel::NewState);
        assert_ne!(abroad.level, AlertLevel::UnconfState);

        // The same ref on a call the resolver places in the US still works,
        // or the guard would have switched WAS off altogether.
        let mut home = matrix();
        home.record(291, "20M", "DATA", "K9XX", true);
        home.record_state("OH", "20M", Some("DATA"), true);
        let at_home = classify_refs(
            &home,
            "K1ABC",
            14.074,
            &cfg,
            AwardRefs {
                state: Some("AZ"),
                ..AwardRefs::NONE
            },
        );
        assert_eq!(at_home.level, AlertLevel::NewState);
    }

    #[test]
    fn the_rarest_candidate_wins_and_filters_shift_the_pick() {
        let cfg = awards_config();
        let mut m = matrix(); // K* (US) never worked → NEW DXCC
        m.record_state("OH", "20M", Some("DATA"), true); // axis live, TX still new
        let refs = AwardRefs {
            state: Some("TX"),
            ..AwardRefs::NONE
        };
        // New DXCC outranks New State.
        let c = classify_refs(&m, "K1ABC", 14.074, &cfg, refs);
        assert_eq!(c.level, AlertLevel::NewDxcc);
        assert_eq!(c.award_ref, None, "the DXCC pick names no award key");
        // NEW DXCC switched off: the same spot flags as its next truth
        // rather than vanishing.
        let mut no_dxcc = awards_config();
        no_dxcc.alert_new_dxcc = false;
        let c = classify_refs(&m, "K1ABC", 14.074, &no_dxcc, refs);
        assert_eq!(c.level, AlertLevel::NewState);
        // Award levels off (the default config): refs are inert.
        let c = classify_refs(
            &m,
            "VU2XYZ",
            50.313,
            &AlertConfig::default(),
            AwardRefs {
                grid: Some("MK97"),
                ..AwardRefs::NONE
            },
        );
        assert_ne!(c.level, AlertLevel::NewGrid);
    }

    #[test]
    fn unknown_mode_answers_only_the_band_half() {
        let cfg = AlertConfig::default();
        // The matrix has India on 20M-DATA (confirmed) and 40M-CW.
        //
        // An empty mode used to reach `canonical("")` and come back DATA, so
        // this spot was scored as the worked 20M-DATA slot — a phone spot
        // silently credited to a digital one. Now the mode half is simply
        // not answered: 20M is worked and confirmed, so nothing is flagged.
        assert_eq!(
            classify("VU2ZZZ", 14.200, "", &cfg).level,
            AlertLevel::Worked,
            "no mode: cannot claim a new mode or slot on a worked band"
        );
        // A band gap is mode-independent, so it still reports.
        assert_eq!(
            classify("VU2ZZZ", 21.200, "", &cfg).level,
            AlertLevel::NewBand,
            "no mode: a never-worked band is still a never-worked band"
        );
        // So is a whole-entity gap.
        assert_eq!(
            classify("K1JT", 14.200, "", &cfg).level,
            AlertLevel::NewDxcc
        );
        // 40M-CW is worked but unconfirmed, so the band is unconfirmed too —
        // visible only with the Unconf half switched on, since it is off by
        // default and `apply_filter` downgrades a disabled level to Worked.
        let unconf = AlertConfig {
            alert_unconf_band: true,
            ..AlertConfig::default()
        };
        assert_eq!(
            classify("VU2ZZZ", 7.150, "", &unconf).level,
            AlertLevel::UnconfBand
        );
        assert_eq!(
            classify("VU2ZZZ", 7.150, "", &cfg).level,
            AlertLevel::Worked,
            "same spot, default config: the disabled level reads as Worked"
        );
        // Contrast: the SAME frequency WITH a mode does reach the mode half.
        assert_eq!(
            classify("VU2ZZZ", 14.200, "SSB", &cfg).level,
            AlertLevel::NewMode,
            "a known mode still gets the full ladder"
        );
    }

    #[test]
    fn level_priorities() {
        let cfg = AlertConfig::default();
        // Unknown entity → new DXCC.
        assert_eq!(
            classify("K1JT", 14.074, "FT8", &cfg).level,
            AlertLevel::NewDxcc
        );
        // Worked slot (FT8 → DATA on 20M).
        assert_eq!(
            classify("VU2ZZZ", 14.074, "FT8", &cfg).level,
            AlertLevel::Worked
        );
        // Never-worked band beats everything.
        assert_eq!(
            classify("VU2ZZZ", 21.074, "FT8", &cfg).level,
            AlertLevel::NewBand
        );
        // Known band + known mode, new combination → new slot.
        assert_eq!(
            classify("VU2ZZZ", 7.074, "FT8", &cfg).level,
            AlertLevel::NewSlot
        );
        // Never-worked mode on a worked band → new mode.
        assert_eq!(
            classify("VU2ZZZ", 14.2, "SSB", &cfg).level,
            AlertLevel::NewMode
        );
    }

    /// All eight on: the `New*` half must still win wherever it applies, and
    /// the `Unconf*` half only speaks for slots that ARE in the log.
    fn all_on() -> AlertConfig {
        AlertConfig {
            alert_unconf_dxcc: true,
            alert_unconf_slot: true,
            alert_unconf_band: true,
            alert_unconf_mode: true,
            ..AlertConfig::default()
        }
    }

    #[test]
    fn unconfirmed_levels_sit_under_the_new_ones() {
        let cfg = all_on();
        // 40M CW is worked but unconfirmed. India HAS a confirmation (20M
        // DATA), so this is not the entity-wide gap — it is the band gap.
        assert_eq!(
            classify("VU2ZZZ", 7.028, "CW", &cfg).level,
            AlertLevel::UnconfBand
        );
        // 20M DATA is confirmed → nothing to chase.
        assert_eq!(
            classify("VU2ZZZ", 14.074, "FT8", &cfg).level,
            AlertLevel::Worked
        );
        // A band never worked at all still outranks any confirmation gap.
        assert_eq!(
            classify("VU2ZZZ", 21.074, "FT8", &cfg).level,
            AlertLevel::NewBand
        );
    }

    #[test]
    fn entity_with_nothing_confirmed_reads_as_unconf_dxcc() {
        let mut m = LogMatrix::default();
        m.record(324, "20M", "DATA", "VU2AAA", false); // worked, never confirmed
        let r = resolver();
        let cfg = all_on();
        let c = AlertClassifier {
            matrix: &m,
            resolver: &r,
            config: &cfg,
        }
        .classify("VU2ZZZ", 14.074, "FT8");
        assert_eq!(c.level, AlertLevel::UnconfDxcc);
    }

    #[test]
    fn unconf_mode_and_slot_are_distinguishable() {
        let mut m = LogMatrix::default();
        // 20M DATA confirmed; 20M CW and 40M DATA worked but not confirmed.
        m.record(324, "20M", "DATA", "VU2AAA", true);
        m.record(324, "20M", "CW", "VU2BBB", false);
        m.record(324, "40M", "DATA", "VU2CCC", false);
        let r = resolver();
        let cfg = all_on();
        let go = |mhz, mode| {
            AlertClassifier {
                matrix: &m,
                resolver: &r,
                config: &cfg,
            }
            .classify("VU2ZZZ", mhz, mode)
            .level
        };
        // 20M is a confirmed band and CW is an unconfirmed mode → mode gap.
        assert_eq!(go(14.030, "CW"), AlertLevel::UnconfMode);
        // 40M band unconfirmed → band gap outranks the mode gap.
        assert_eq!(go(7.074, "FT8"), AlertLevel::UnconfBand);
    }

    #[test]
    fn unconfirmed_levels_are_off_by_default() {
        // A fresh install behaves exactly like 1.x.
        let cfg = AlertConfig::default();
        assert_eq!(
            classify("VU2ZZZ", 7.028, "CW", &cfg).level,
            AlertLevel::Worked,
            "the ? levels must stay silent until switched on"
        );
    }

    #[test]
    fn filters_downgrade_to_worked() {
        let cfg = AlertConfig {
            alert_new_band: false,
            ..AlertConfig::default()
        };
        assert_eq!(
            classify("VU2ZZZ", 21.074, "FT8", &cfg).level,
            AlertLevel::Worked
        );
    }

    #[test]
    fn beacons_and_gaps_never_alert() {
        let cfg = AlertConfig::default();
        let c = classify("4X6TU", 14.1, "CW", &cfg);
        assert_eq!(c.level, AlertLevel::None);
        assert!(c.is_beacon);
        assert_eq!(
            c.dxcc_name.as_deref(),
            Some("NCDXF Beacon — Tel Aviv, Israel")
        );
        // Frequency outside any band → None level, but DXCC still resolved.
        let c = classify("VU2ZZZ", 2.5, "FT8", &cfg);
        assert_eq!(c.level, AlertLevel::None);
        assert_eq!(c.dxcc_id, Some(324));
        // Empty call → nothing.
        assert_eq!(classify("", 14.074, "FT8", &cfg).level, AlertLevel::None);
    }

    /// WSPR spots reached the Telegram alerts as New Band/Mode catches.
    /// The same station on FT8 alerts; on WSPR or FST4W it must not.
    #[test]
    fn wspr_spots_never_alert() {
        let cfg = awards_config();
        assert_ne!(
            classify("VU2ZZZ", 21.074, "FT8", &cfg).level,
            AlertLevel::None
        );
        for mode in ["WSPR", "wspr", "FST4W"] {
            let c = classify("VU2ZZZ", 21.0946, mode, &cfg);
            assert_eq!(c.level, AlertLevel::None, "{mode}");
            assert_eq!(c.dxcc_id, Some(324), "entity still resolves");
            assert_eq!(c.award_ref, None);
        }
        // A new entity on WSPR is still no alert.
        assert_eq!(
            classify("4X1ZZ", 14.0956, "WSPR", &cfg).level,
            AlertLevel::None
        );
    }
}
