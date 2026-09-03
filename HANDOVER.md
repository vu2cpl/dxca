# DXCA — Project Handover
*For continuation in a new Claude session*

**Created:** 2026-08-26 · **Last updated:** 2026-09-03 · **Status:**
**v2.20.4 — the Zone and Marathon rows in Alerts were checkboxes wired to
nothing** (Manoj: *"if dx marathon is not selected in alerts, still
notifications were coming"*, *"have to disable in awards to stop the
notifications"*, *"dx marathon if selected, selects zone also"*). Two
symptoms, one fault, and the second one names it exactly.

`Alerts.svelte` kept its own `FIELD` table of level key → `notify_*` field,
fourteen entries, written when the ladder had fourteen levels.
`AlertLevel::FLAGGABLE` reached **seventeen** in 2.19.0 with `newZone`,
`unconfZone` and `marathon`. Those three keys missed the table, so
`FIELD[l.key]` was `undefined` and all three rows bound to `cfg[undefined]`
— **one shared slot**, which is why the Marathon tick moved both Zone rows.
Worse, the save spread `{...cfg}` and therefore never sent
`notify_new_zone`, `notify_unconf_zone` or `notify_marathon`; all three
carry `#[serde(default = "default_true")]`, so every save put them back ON.
Nothing in the Alerts tab could silence them. The award pair on Settings ›
Awards could, because that is the *classifier* gate — which is exactly the
workaround he found.

**The fix removes the class, not just the three rows.**
`NotifyUserConfig::notify_field` lives beside `wants_level` in `db.rs`,
`/api/reference` serves it on each level as `notifyField`, and the Alerts
tab binds to that instead of a table of its own — the same reason the level
list, the band table and the mode buckets are served rather than retyped. A
level arriving without a field is filtered out of the ladder: a control that
cannot say no is worse than no control. Three tests: every flaggable level
names a field that exists on the struct *and* that `wants_level` reads (both
directions), no two levels share a field, every served level carries one.
Gate green, 293 tests.

**The classic-eight `DXCC_FIELD` table in `AwardSettings.svelte` was checked
and is complete** — that set does not grow, and the award levels there come
from the `AWARDS` array, so this fault has no second home.

**Tagged, released and on all five hosts, 2026-09-01.** Bump before the
deploy this time, so one round did it: local Pi first and verified
(`/api/reference` there returns 17 levels, 17 with a `notifyField`), then
`--no-seed` to `adersh`, `vu2wj` and `vu2oy`, then `win-deploy.sh`. Sweep
reads **v2.20.4** on all five, every cluster node reconnected (9/9, 4/4,
2/2, 2/2, 2/2), and each host serves the full ladder. Windows still reads
`cty_entities: 0`, the accepted state. Release page carries
`dxca-2.20.4-windows-x64.zip`.

**v2.20.3 — the Awards help tips claimed confirmed always means LoTW,
which is false for WAZ** (Manoj: *"why 2 zones worked and
confirmed"*). Both zone numbers come from the ClubLog export and nowhere
else: worked is any credited QSO carrying a `CQZ`, confirmed is one whose
record has a QSL flag — `LOTW_QSL_RCVD`, `QSL_RCVD`, `EQSL_QSL_RCVD` or
`APP_CLUBLOG_QSO_QSL` (`adif.rs::is_confirmed`). `merge_lotw_confirmed`
writes grids, states and islands only, never zones, so WAZ scores fully
without a LoTW login — which the WAZ card's own tip already said, while the
tip two cards above it and the Awards settings tip said the opposite. Both
now split the rule per award. Text only; the counts were always right.

**Deployed to noderedpi4 on 2026-09-01** (`deploy/pi-deploy.sh`, seeded
form, service back clean with all nine cluster nodes Live and the telnet
client reconnected; bundle `index-LDTEEEYk.js` serves the corrected tip and
no longer carries the old sentence). Local Pi first, ahead of any tag.

**Then the other four, the same day** — `--no-seed` to `adersh`, `vu2wj`
and `vu2oy` (all three tunnels were already up, all three boxes powered and
on 2.20.2 with no telnet clients to drop), `win-deploy.sh` to the Windows
box. All five now serve `index-LDTEEEYk.js` with the new wording and every
cluster node reconnected: 9/9 on noderedpi4, 4/4, 2/2, 2/2, 2/2. Windows
still reads `cty_entities: 0` — the known, accepted state from 2026-08-30,
not a deploy fault.

**Tagged, released and re-shipped the same day.** The first round went out
from unreleased main, so all five served the fix while reporting 2.20.2;
after the bump they were redeployed in the same order (local Pi, verified,
then the three `--no-seed` boxes, then Windows) and **`/api/status` reads
2.20.3 on all five** — 9/9, 4/4, 2/2, 2/2, 2/2 cluster nodes back, telnet
client back on noderedpi4. The Windows zip is on the release page; `gh
release create --latest` also moved the *Latest* label, which had been stuck
on v2.20.1 since the tags were published out of order.

**v2.20.2 — one type scale on the Awards tab** (Manoj: *"each portion has
different font sizes"*). He was right, and the first fix was wrong: I
added a `--fs-value` token, when `app.css` states its four-role scale is
declared once **"so a screen cannot invent a fifth size"**. Data now takes
the existing `--fs-item`, and the section headings are the plain `<h2>`
the app already uses for a second heading inside a card — my `.sub` class
had been re-implementing that badly at a magic 0.62rem. Two dead rules
(`.sub`, `.tally*`) went with it.

WAZ by-band and by-mode also carry **both** numbers now: unlike WAS, where
worked and confirmed cannot differ (only LoTW writes states), zones come
from the ClubLog log and do — his 60M is 26 worked / 24 confirmed, 10M is
39 / 38, and that gap is the chase.

**v2.20.1 — WAZ gains a mode split and an honest confirmed count.** Manoj:
*"waz doesnt have modewise split ... and it doesnt show mixed waz numbers,
just says all worked, is it confirmed?"* — three faults, and the third was
the sharp one: the summary printed `waz_worked / 40` in the **confirmed**
style, so a worked count was being read as a confirmed one. It now shows
both, and the WAZ card mirrors the WAS endorsements card: by mode, a
*Still needed* worklist (mixed first, then each mode), and by band.
`waz_missing` is now **confirmation-based** — an award is claimed on
confirmations, so a worked-but-unconfirmed zone is still wanted.
`WazScope::PerMode` joins Mixed and PerBand.

His data, for reference: 40/40 worked **and** 40/40 confirmed mixed, so the
answer to the question was yes — but CW 40, Phone 38 (missing 2, 6) and
Data 39 (missing 2), which makes per-mode a live chase where mixed is done.

**v2.20.0 — more than one FlexRadio and more than one ExpertSDR3 per
account.** VU3ESV's PR #2, reviewed against main at v2.19.3 and merged:
clean merge, full gate green on the merged result (290 tests), and both
halves verified present afterwards — his multi-radio loops and the
WAZ/Marathon levels that had landed in the same `match` arms hours
earlier.

**What made it safe to take:** `NotifyUserConfig` carries
`#[serde(default)]` at struct level *and* on both new lists, and
`notify_config` adopts a pre-list `flex_host`/`tci_host` into a one-entry
list — the same move `notify_spotter_kind` makes for the legacy
`notify_manual_only` boolean. `set_notify_config` writes the legacy pair
back from the first device and **blanks it when the list is emptied**, so
deliberately removing every radio cannot resurrect the old address on the
next load. Both are tested.

**The interesting part is the dedupe**: `radio_targets` resolves the port
*before* comparing, so `192.168.1.60` with a blank port box and
`192.168.1.60:40001` are one radio typed two ways rather than two marks
per spot — and on TCI, two `SPOT_DELETE`s racing to remove one mark. A
different port on the same host stays two targets, which is two instances
on one machine. One helper shared by both radios, because that rule
drifting would be invisible until someone got double marks.

**v2.19.3 — "Still needed" covers mixed WAS too** (Manoj: *"also lacks
mixed was still needed data"*). The per-mode gaps were listed but the
mixed list sat in the summary card as "Missing states: …", so the one
section that answers *which states do I still want* did not answer it for
basic WAS. Mixed is now the first row of that list and the summary no
longer repeats it — the same one-place rule the duplicated Triple Play
score got in 2.19.1.

**v2.19.2 — the Awards tab reads as sections** (Manoj, on the second look:
*"still bad"*). The real fault was not spacing but **vocabulary**: `.sub`
was bold body text at 0.95rem, so "Still needed" and "States per band"
read as sentences among the numbers instead of as dividers. They now use
the app's own section-heading style — uppercase, 0.62rem, letterspaced,
muted — with a hairline rule and real space above, and the first one in a
card takes the space without the rule. The band runs also became the same
label-over-value `dl.stats` the mode counts use, so the card counts things
one way rather than two.

**v2.19.1 — the Awards tab is tidied** (Manoj: *"needs uncluttering
only"*). The "still needed" list was a `display: grid` with
`grid-template-columns: auto 1fr` being fed **wrapper `<div>`s**, so each
mode was one cell: two modes landed per row and the third was orphaned
below them. `dt`/`dd` are the grid's own children now, one mode per row,
ordered CW/Phone/Data like every other mode list. Triple Play's score was
also printed twice — summary card and endorsements card — and is now only
in the summary.

**v2.19.0 — WAZ, the DX Marathon, and a WAS scope you pick.** Manoj:
*"awards on stats is not what i want ... i need in settings to select
whether to chase a WAS triple play or as per band"*, then *"lets roll waz
and dx marathon"*.

**The correction that matters:** an award variant is a **chasing** setting,
not a reporting one. `WasScope` (mixed / triple / band) and `WazScope`
(mixed / band) change what the classifier calls *new*, so a state long
since worked still alerts in a mode or on a band it is missing from. Same
levels, same ladder — only the question changes, so no new rows anywhere.

**The zone foundation, which both new awards stand on.** `PrefixRule` was
parsing cty.xml's `<cqz>` and throwing it away, so zones could only ever
be entity-level. `DxccResolver::zone()` now walks the same
exception → prefix → entity ladder `resolve` does: VE7 → 3, VE3 → 4,
UA9 → 17, UA3 → 16, all verified against the live cty.xml.

**But cty.xml has NO US call-area records** — every US call resolves to the
entity zone, 5. For a zone award that is fatal: zones 3 and 4 would be
uncreditable. So `awards::us_zone()` maps state → zone (3 west, 4 middle,
5 seaboard, AK 1, HI 31) and the server prefers it for US calls, reusing
the FCC table already loaded for WAS. Without that table, a warning now
says so on the Awards page rather than letting the award quietly under-report.

**Marathon** is the first axis with a **time dimension** (`by_year`), since
it resets every January, and the year comes from the *spot's* timestamp
rather than the clock so a spot processed either side of midnight on 31
December scores in its own year. It is one level, not a New/? pair: the
award scores what you worked, so there is no confirmation gap to chase.

**v2.18.0 — WAS endorsements, Triple Play, and Awards as its own Stats
tab** (Manoj: *"add band wise and mode wise WAS as a new award tab —
triple play?"*, then *"triple play is WAS in all 3 modes, not band"*).

`AwardStatus` gains a **mode axis** (`modes` / `confirmedModes`,
serde-defaulted so stored matrices still load), fed from the ADIF `MODE`
through the same `modes::canonical` bucketing the DXCC slots use — LoTW's
own `APP_LoTW_MODEGROUP` agrees with it, so one bucketing rule governs
everything. `triple_play_count` and `triple_play_missing` sit on the
matrix; the latter is the worklist, which is the half a score cannot give.
**Triple Play's LoTW-only requirement is met by construction**, not by a
check: `merge_lotw_confirmed` is the only thing that ever writes a
confirmed state, because ClubLog's export has no `STATE`.

**The correctness fix found while measuring it.** Asked for his real
numbers, the naive count gave 49 states — one of them `SD`, credited by a
**CHINA** QSL, because Shandong's subdivision code is also SD. LoTW's
`STATE` is worldwide and several codes collide with US ones (SC = Santa
Catarina, MO = Moscow oblast, MT/MS/AL/PA all have Brazilian twins). The
spot side has been gated on a WAS-countable entity since 2.17.3; **the log
side never was**. Both merge paths now gate on `counts_for_was`, and the
regression test carries the exact China/SD record.

His real position after the gate: **48 states**, CW 44 / Data 46 / Phone
43, **Triple Play 39 of 50**.

**Next, agreed with Manoj and explicitly step-by-step:** the **DX
Marathon** (entities + CQ zones in a calendar year, no band/mode) and
**WAZ** (40 zones, per band). Both are feasible from data already
downloaded — ClubLog's export carries `CQZ` on 56,283 of 56,845 records,
and cty.xml's `DxccEntity.cq_zone` has been parsed-but-unused since 2.0.
Marathon needs the one axis the matrix has never had: **time**, keyed by
QSO year.

**v2.17.7 — "All" owns the first line of every chip group** (Manoj:
*"keep all in one line and rest in the next lines, will look cleaner"*,
and *"even in modes"*). One change in `ChipGroup.svelte` covers Sources,
Alerts, Modes and Bands on both rails, because they share the component.
The options moved into a `flex-basis: 100%` wrapper rather than getting a
zero-height spacer element: a spacer is its own flex item and would have
produced two stacked row-gaps under All instead of one.

**v2.17.6 — the LoTW download gets time to finish, and a failed one stops
erasing.** 2.17.5 fixed the *request* (it was incremental) and the very
first real run then failed a different way: `LoTW report read: timed out
reading response`. The 600 s timeout was not enough for a 28,467-record
report that LoTW builds server-side before sending a byte. Raised to
**1800 s**.

**The more important half is the second fix.** When the download failed,
`refresh_user` saved the freshly rebuilt matrix with `by_state` and
`by_iota` EMPTY — the observed `byIota` went 2 → 0 — because the rebuild
starts from scratch and the merge simply did not happen. That is the same
erasure as the incremental bug wearing a different hat, and it means *any*
LoTW hiccup silently republishes "you have worked no states". The failure
path now **carries the previous axes forward**; `by_grid` is not carried,
because it is rebuilt from the ClubLog log we just parsed and the fresh
value is the correct one.

Third: *Refresh log now* said "this can take a minute" while the operator
watched a spinner for fifteen. It now says **10 minutes or more** when LoTW
credentials are set, and that it keeps running if the page is left.

**Pattern worth naming, three for three today:** every one of these bugs
was a rebuild-from-scratch matrix meeting a data source that returned less
than expected. The `?`-axis guard (2.17.5) and this carry-forward are the
two structural answers; the individual download bugs were only triggers.

**v2.17.5 — the LoTW report is actually fetched in full.** Manoj: *"getting
new state alert for NK3L CA, but I have worked CA"*. His matrix held
`byState: 0`, `byIota: 2`, `byGrid: 594` — grids healthy (they come from
ClubLog), states and islands empty.

**Cause:** LoTW's QSL report is **incremental by default** — it returns the
QSLs received since your last download — and `lotwreport::download` never
pinned a start date. The first fetch brought the history; a later one
brought two records and overwrote the cache; and because `refresh_user`
rebuilds the matrix from scratch and merges whatever it gets, every state
and island earned before that vanished. Measured against the live endpoint
with his credentials: the request as it stood returned **1 record, 0 STATE
fields, 871 bytes**; with `qso_qslsince=1945-01-01` it returned **28,467
records, 6,786 STATE fields, 16.8 MB**.

**The module doc had stated the requirement** — *"Always a FULL report ...
an incremental pull would lose every older LoTW-only confirmation on the
next rebuild"* — and the code did not implement it. A comment asserting a
property is not a test of it.

**Second fix, for the whole class:** an empty award axis now claims nothing
is new (`best_award`). States and islands have exactly one source, an
optional external report that can be absent, refused or — as here —
quietly partial, so an empty map means *unknown*, not *none worked*.
`by_grid` is deliberately NOT guarded: it comes from the same ClubLog log
that drives DXCC, so empty there really does mean none worked. Two of the
2.17.3 tests had to be repaired to keep testing what they claimed — their
fixtures had empty axes, so they would have passed on the new guard rather
than on the rule they were written for.

**Verified working 2026-09-01 after the 2.17.6 fixes**: a refresh on
noderedpi4 pulled the full 16.8 MB report and the matrix now holds **49
states and 319 IOTA groups** (missing: WY only, which is a genuine gap in
the log, not a bug). `CA` present, so the NK3L report is closed.

**Operator step after upgrading: one Refresh log now**, or states stay
empty until the daily refresh comes round. **The poisoned 1,431-byte cache
was deleted from noderedpi4** during the 2.17.5 deploy — without that the
week-long cache would have re-merged the bad file and re-erased the states
even on the fixed build. No other host had a cached report (no LoTW
credentials set on them).

**v2.17.4 — the Server card and the file-only line stop lying.** Three
display defects Manoj found by reading the Reference-data page: the
**Milestone** row was a hardcoded `"2.1 — alerts, awards, auto-refresh"` in
`status_json` that nothing ever updated, printing 2.1 beside a version
reading 2.17 — removed from the API and the card, and nothing else consumed
the field. The **file-only settings line** omitted `iota_refresh_days` and
`fcc_refresh_days`, added to the config in 2.17.0 and never added to the
line, so it under-reported what the TOML owns. And **`telnet_interactive`
was invisible from the web UI entirely** — a flag that changes what port
7575 accepts, readable only by opening the TOML on the box; it is now in
`read_only` and on the line as `telnet 7575 (login on/off)`. Intervals of
`0` render as `off` rather than `0d`, which read as "constantly".

**The lesson for that line specifically:** it is a promise of completeness,
so any new `Config` field must be added to it in the same commit. It went
wrong the moment a config key shipped without one.

**v2.17.3 — a US call operating abroad is no longer a New State.** Manoj
spotted `DV2/K7AZQ` flagged as one: an Arizona licensee transmitting from
the Philippines. **The cause is the part worth remembering** —
`StateTable::lookup` reused the exact / before-slash / after-slash ladder
that `lotw::is_user` and `has_worked_call` walk. That ladder answers *who
does this call belong to*, where finding `K7AZQ` inside `DV2/K7AZQ` is
exactly right; it was being asked *where is this operator*, where it is
exactly wrong. **A test asserted the broken behaviour** (`KH6/K6XYZ` →
`CA`, labelled "prefix override"), which is how it shipped — a test can
only protect a rule someone stated correctly.

Two guards now, deliberately redundant: `best_award` requires the spot's
resolved entity to be WAS-countable (291 / 6 / 110 — Alaska and Hawaii are
WAS states though separate DXCC entities), and `lookup` itself takes only
an exact match or a plain `/P`, `/M`, `/QRP`. A call-area digit (`W1AW/7`)
is refused too: that suffix is the operator *saying* they are not home.
Also in 2.17.3: the settings pages are just **ClubLog** and **LoTW**.

**v2.17.2 — explanatory prose lives on the `?` hovers** (Manoj: *"move all
explanatory texts to on hover"*), continuing the rule `HelpTip.svelte`'s own
header states: read-once paragraphs were pushing the daily controls down the
page. Moved: each award's "what it runs on" (Awards), the LoTW "what it is
for" intro, the ClubLog ladder/LoTW pointer, and both `.always` reassurances
in the Alerts rail — the band-mask group gained a HelpTip to hold its New
DXCC exemption, **overriding an earlier deliberate choice** to keep that one
on the page ("a tooltip nobody hovers"); say so if it should go back.
**Kept visible:** conditional warnings and state (no FCC table, no IOTA
directory, no levels ticked, LoTW not set) — those are not explanation.

**v2.17.1 — ClubLog and LoTW are separate settings pages** (Manoj, same
day: *"my clublog and my lotw to be different tabs"*). Two accounts at two
organisations; the LoTW login was a footnote under the ClubLog form and
easy to miss. **UI split only** — both pages still edit the one
`clublog_json` row and both write it back whole, so a partial PUT cannot
blank the other's credentials. No schema change, deliberately.

**v2.17.0 — awards beyond DXCC, and a settings page to pick them.** IOTA,
WAS and VUCC each get a New/`?` level pair, switched on under **Settings ›
My station › Awards**; an award left unticked adds no control anywhere, so
the Spots and Alerts screens of a non-chaser are unchanged. Also in the
release: the confirmation-path gate on the `?` levels, the TCI (ExpertSDR3)
destination with its reconnect defect fixed, and the ClubLog embed
following the app's appearance. **The whole fleet is on v2.20.2 (2026-09-01)** — all five hosts, every
cluster node reconnected, verified after the deploy. `vu2wj` had needed a
retry on the v2.20.0 pass: its first attempt died at `ssh: connect ...
Operation timed out` *before* anything transferred, so it sat untouched
rather than half-upgraded, and came back on its own minutes later. Same
for `adersh` later the same evening. Both were the **stale-handshake
signature** — the `/32` route present the whole time while ping and ssh
failed — not powered-off boxes. **Retry a failed tunnelled deploy once
before bouncing a tunnel or concluding the box is off.**

**The Windows box downloaded the IOTA directory by itself** (1178 groups)
on the scheduler's first tick, while `fcc_calls` stayed 0 — which is
exactly the intended asymmetry: the 290 KB directory is automatic, the
~200 MB FCC dump is manual-first.

**v2.17.0 through v2.17.7 all released 2026-09-01** (tag + GitHub release
+ Windows zip each). **noderedpi4 runs v2.17.7**; the other four are on
**v2.17.5** — 2.17.6 (LoTW timeout + carry-forward) and 2.17.7 (chip
layout) are inert on hosts with no LoTW credentials, and re-restarting
third-party feeds for them was not worth it. The table below is the
2.17.5 fleet pass —
deployed and verified the same day, every host answering with its nodes
live:

| Host | Nodes | cty / IOTA / FCC |
|---|---|---|
| `192.168.1.169` noderedpi4 | 9/9 | 402 / 1178 / 816973 |
| `192.168.1.170` Windows | 2/2 | 0 / 0 / 0 |
| `192.168.1.151` adersh | 4/4 | 402 / 0 / 0 |
| `192.168.1.201` vu2wj | 2/2 | 402 / 0 / 0 |
| `192.168.220.51` vu2oy | 2/2 | 402 / 0 / 0 |

**The award reference files are noderedpi4's alone**, and that is correct,
not an incomplete deploy: `pi-deploy.sh --no-seed` ships no data, and the
FCC table is a ~200 MB download that must stay each admin's own deliberate
act (Server › Reference data). The four other hosts can tick IOTA/WAS/VUCC
on Awards, but their island and state levels stay quiet until their own
admin downloads the files. Grid (VUCC) works everywhere immediately — it
needs no reference data.

The Windows box's `cty_entities: 0` is unchanged and still the known,
accepted state — do not re-raise it.

Bringing the three WireGuard tunnels up needs a sudo password, so it is
**Manoj's step, not an automatable one**: `sudo wg-quick up Adersh_vu2cpl`
/ `Shaji_vu2wj` / `vu2cpl_Ranjith`, bare names from
`/opt/homebrew/etc/wireguard/`. All three ran up together with the shack
LAN, as designed since the AllowedIPs narrowing.

**All five hosts ran v2.16.0 as of 2026-08-30**, and four still do —
noderedpi4 is the only one on v2.17.0 so far. `adersh@192.168.1.151` was the
last one — it had missed the deploy pass with `ssh: connect to host
192.168.1.151 port 22: Operation timed out`, which read as the third-party
power-state case but was **not**: the Pi was up and its `192.168.1.151/32`
route was in the table the whole time, and the WireGuard *handshake* had gone
stale. `sudo wg-quick down Adersh_vu2cpl; sudo wg-quick up Adersh_vu2cpl` on
the Mac brought it back within 20 seconds, and
`deploy/pi-deploy.sh --no-seed adersh@192.168.1.151` then ran clean.

**Diagnostic note for next time:** a tunnelled host that fails both ping and
ssh while `netstat -rn` still shows its /32 route is the handshake, not the
box. Bounce the tunnel before concluding the far end is powered off. Verify
the fleet with `curl -s http://<host>:7580/api/status` — it returns `version`
without an ssh session, and the binary has **no `--version` flag** (running
`/opt/dxca/dxca --version` starts a second server and dies on the port clash).

**Windows `192.168.1.170` reports `cty_entities: 0`** where all four Pis
report 402, so DXCC entity resolution is dead on that install. Raised
2026-08-30 and **deliberately left alone** on Manoj's instruction — a known
accepted state, not an open bug. It also **blocks ICMP**, so a failed ping
there is not evidence the host is down; it serves `/api/status` fine.

**v2.15.1 — alerts on the FlexRadio panadapter**, over the SmartSDR API on
TCP 4992, colour-coded by level and expiring on a per-level ladder (DXCC an
hour, Band/Mode 15 min, the rest 1). Per-account, off by default, alerts only.

**v2.14.0 — health alerts.** Telegram when DXCA is up and nothing is reaching
it: feed quiet, or a node disconnected. Both opt-in per account, off by
default, and honest about what they cannot do (a dead host cannot report
itself). See the entry under Open items.

Earlier the same day, **v2.13.1 → v2.13.3**, all released and deployed to all
five hosts. The headline: award totals now agree with ClubLog's own, after
two separate cty.xml rules turned out to be unread — and every uncredited
contact is named in the log at refresh time. **Each account still needs one
ClubLog refresh** before its totals move. **All four have now had theirs
(2026-08-30)**, so every log in the fleet is scored under the corrected
rules:

| Account | Worked | Uncredited found |
|---|---|---|
| VU2CPL | 320 | 3 (V55DX) |
| VU24DX | 313 — matches the ClubLog dashboard | 18 |
| VU2WJ | 314 | 24 |
| VU2OY | 275 | 1 (`SV2RSG 20211201 014315Z 40M FT8`) |

VU2OY's single hit is the bare `SV2RSG`, invalid from 2016-04-21 onward — a
different entry from the three dated `SV2RSG/A` windows, and a true positive
on a December 2021 QSO.

Previous status block:
**v2.13.1 TAGGED, RELEASED AND ON ALL FIVE HOSTS (2026-08-30).** Two changes:
ClubLog's **invalid-operations list** is finally honoured (DXCC totals now
agree with ClubLog's own — see the entry under Open items) and Telegram alerts
mark **LoTW stations with an asterisk**. Release published with the Windows
zip. All five report v2.13.1 on `/api/status`: noderedpi4, Windows
`192.168.1.170`, adersh, vu2wj, vu2oy.

**DEPLOYING IS NOT ENOUGH ON ITS OWN, and this bit surprises.** The fix
changes how a matrix is *built*; stored matrices are untouched by an upgrade,
so an upgraded host shows **exactly the old numbers** until each account runs
**Settings › ClubLog account › Refresh log now**. Manoj hit this within
minutes of the deploy — VU24DX still read 314 against ClubLog's 313 on a host
already running the fix. Auto-refresh (default 24h) gets there on its own;
the button is the same thing, now. Any future change to matrix *building*
carries this same footgun — say so in the release notes.

The check that proves the fix: VU24DX on `adersh@192.168.1.151` should read
**313** after that refresh, matching the ClubLog dashboard lower down the same
page. That host's cty.xml carries all 2,838 invalid operations including the
three `SV2RSG/A` windows, confirmed on the box.

**The Windows box has no cty.xml, and that is fine — it is a TEST box.**
`/api/status` reports `cty_entities: 0` and `C:\DXCA\data` holds only
`dxca.db` and `lotw-users.txt`. Confirmed by Manoj 2026-08-30: no real log,
no real account, it exists to prove the Windows build installs and runs. So
it cannot classify DXCC and none of the award work applies there — **not a
defect, do not "fix" it** by adding an API key. Deploy to it to check the
binary, and read nothing into its zeroes.

Previous status:
**v2.12.1 on ALL FOUR HOSTS (2026-08-29 evening)** — noderedpi4, Windows
`192.168.1.170`, `adersh@192.168.1.151`, `vu2wj@192.168.1.201`. Three changes
on top of the shell rework: the **network-failure fix** (a dropped route no
longer makes the Settings pages look like lost configuration), the **ClubLog
DX Dashboard** embedded under *Stats › My ClubLog*, and **Stats filling the
window** instead of stopping at 56rem. Every host was deployed from the
**v2.12.1 tag itself**, checked out detached, so all four run byte-identical
released code — `main` carries one commit past it (a dead-code removal in the
embed's callsign fallback) that is deliberately unshipped.

Both VPN hosts kept their own `config/dxca.toml` byte-for-byte — md5 taken
before and after on each, which is the check that actually proves `--no-seed`
did its job.

**The ClubLog embed is confirmed working in a real browser** (Manoj, Safari).
It rendered BLANK in the sandboxed preview pane with no console error and
clublog.org reachable, which was the sandbox blocking the dashboard's own
subresources (`cdn.clublog.org`, `unpkg.com`) — not a defect. **Lesson worth
keeping: the preview pane cannot verify third-party iframes.** A blank embed
there is not evidence of anything; check it in Safari against the Pi.

**v2.12.0 — THE SHELL REWORK, on ALL FOUR HOSTS (2026-08-29).** noderedpi4,
Windows `192.168.1.170`, `adersh@192.168.1.151` and `vu2wj@192.168.1.201`. The
two VPN hosts report v2.12.0; the two LAN hosts run this exact code but were
deployed from `0ea73aa`, before the version bump, so they still REPORT v2.11.1
in the header — a cosmetic lag, fixed by redeploying either at leisure.

The `snr_db` migration ran on all four and is confirmed working end to end:
vu2wj recorded an alert WITH an SNR within minutes of the upgrade (87 rows, 86
of them the pre-migration NULLs). Both VPN hosts kept their own
`config/dxca.toml` byte-for-byte — vu2wj's md5 was checked before and after. The two VPN hosts —
`adersh@192.168.1.151` and `vu2wj@192.168.1.201` — are **still on v2.11.1** and
update one at a time on Manoj's prompt, both `--no-seed`. This is the UI
cleanup pass that the previous session left as the next item, and the largest
single change the web UI has had. Three
tabs (**Spots · Alerts · Stats**) plus a **gear** into Settings, Meridian's
arrangement; every setup screen moved behind it; the Spots and Alerts feeds
rebuilt on a fixed measured grid with a collapsible filter rail. Four Rust
files changed with it, including **a schema migration that has already run
against production** (`snr_db` on `alerts_sent`). See the session entry below
for the whole list, and "Open items" for what a release still needs.
Previously **v2.11.1 on ALL FOUR HOSTS** (2026-08-29) — UI polish on the mask and the
station card, all from Manoj looking at the running screen: the mode selector
reads **dim / hide**, the Telegram tick leads with **Band mask** and states
the New DXCC exemption on its own label, and **include deleted entities**
moved under the totals it changes instead of floating at the card's right
edge. Previously **v2.11.0 on all four** (2026-08-29) — the band mask moved to sun
phases (Dawn/Day/Dusk/Night) around a **tunable grey-line window**, default
45 minutes, plus milestone 4: hide mode and the Telegram narrowing. Ported
from Meridian's greyline model so the two programs agree about the phase.
Previously **v2.10.0 on all four** (2026-08-29) — noderedpi4, `adersh@192.168.1.151`,
`vu2wj@192.168.1.201` and now **Windows at `192.168.1.170`**, which joined the
fleet this day. Contents: the phase-rotation band mask (milestone 3, dim mode
with a masked count), the Windows installer's move to a fixed `C:\DXCA`, the
scheduled-task parse bug that had never worked, fixed-width Stats labels, and
`deploy/win-deploy.sh`. Deployed in the standing order — shack Pi first, then
the LAN Windows box, then the VPN hosts one at a time on Manoj's prompt. The
Windows box updated over SSH in one command for the first time; every earlier
Windows update meant unzipping a release by hand. Both Pis took `--no-seed`.

**The band mask is invisible to adersh and vu2wj until each sets a locator on
their own My ClubLog page** — it is per-account and their QTHs are not this
one. Nothing changed on their screens today, which is the intended default. Previously: **v2.9.1 on ALL THREE Pis** — noderedpi4, `adersh@192.168.1.151` and `vu2wj@192.168.1.201`. Every tag from v2.4.0 onward has a published GitHub release with a Windows zip (v2.3.0 and v2.3.1 remain bare tags, superseded by v2.4.0's release notes). **v2.3.0–v2.7.0 all shipped on 2026-08-28**, in order: the interactive telnet gate and read-only command passthrough (`telnet_interactive = true` on noderedpi4, still **false** on adersh); spotter attribution — Source is the feed that carried a spot, Spotter is the station that heard it — carried into Telegram and the My Alerts history, with the **first schema migration** this database has had; a spots search over call/spotter; award totals that count **current DXCC entities by default**, with an *include deleted* tickbox; skimmer identification with a **Manual only** display filter; and Telegram's own *human spots only* narrowing. Both migrations were verified against real data (91 and 102 alert rows preserved), and skimmer/spotter attribution was confirmed live on both stations.
**Repo:** https://github.com/vu2cpl/dxca (**public** — verified via
`gh repo view` 2026-08-27; the doc said "private" until then, and the
"Open items" release checklist still lists the public flip as pending)

---

## What this is

FT8/FT4 + DX-cluster spot aggregator with a multi-user web GUI — Rust
successor to [DXClusterAggregator for
macOS](https://github.com/vu2cpl/DXClusterAggregator-macOS), Pi-first.
**The design and milestone plan is [docs/PLAN.md](docs/PLAN.md) — read it
before touching anything.** It was drafted in the 1.x repo
(`docs/DXCA2-RUST-PLAN.md` there, same content at draft time); this copy is
canonical from now on.

Lineage: original concept by Vinod VU3ESV; DX-cluster telnet client
lifted from `~/projects/meridian` (`crates/meridian-core/src/dxcluster/`),
and the web GUI's design system from the same repo's
`web-ui/default/src/` (app.css + the theme module and switcher).
**Production runs on noderedpi4 (192.168.1.169) since the 2026-08-27
cutover**; the 1.x macOS app is the retained fallback (maintenance mode).

## Session 2026-09-03 — the ClubLog API key ships in the binary

**On `main`, unreleased — still v2.20.4, no tag, nothing deployed.**

A fresh dxca had a cold start that nothing in the UI admitted: alerts and all
DXCC resolution need cty.xml, cty.xml needs a ClubLog API key, and a
`--no-seed` install arrives with **no cty.xml at all** (the file is git-ignored
runtime state; "seeding" copies *this Mac's* data dir, which is exactly what
`--no-seed` exists to prevent). `refresh_cty` hard-errored, `run_cty_if_due`
returned early on the empty key and never self-healed, and `resolve` returned
`None` for everything. So a third-party Pi classified nothing until its admin
went to clublog.org/requestapikey.php and waited for a key to be issued by
hand — for a value that is **the same on every server**, because ClubLog issue
API keys per *application*, not per operator.

The key is now baked into the binary at build time and **is never in this
repository**. ClubLog's
[API Keys](https://clublog.freshdesk.com/support/solutions/articles/54910-api-keys)
article says keys found published on the web or **in a Git repository** are
deleted without notice, with the products using them liable to be blocked, and
this repo is public. Obfuscating a committed key past their scanners would be
the wrong answer to that, so the key never reaches a tracked file at all:

- `crates/dxca-server/build.rs` reads `DXCA_CLUBLOG_API_KEY` or the
  git-ignored `.clublog-api-key`, XORs it against a high-bit pad, and writes
  the bytes into **`OUT_DIR`** — under `target/`, never the source tree. So
  there is nothing to stage, clear or forget, and `just dist` / `just win` /
  `deploy/*.sh` / `install.sh` all pick it up with no change, because they run
  cargo in the same shell. A wrong-length key **fails the build** rather than
  shipping a binary that 403s in the field.
- `crates/dxca-server/src/builtin.rs` de-obfuscates on first use and offers
  `effective_clublog_api_key(&db)` — **an admin-set key still wins**, so an
  operator who prefers their own quota, or a fleet running after ClubLog ever
  revoked the shipped key, needs no new build. Unit-tested, including that
  whitespace is a clear rather than a key.
- The three read sites (`refresh.rs`, `api.rs::cty_refresh`, and the config
  GET) go through it. `GET /api/config/global` still returns **only the
  admin-set key** plus a new boolean `clublog_key_built_in` — the built-in key
  is deliberately never sent to a client, or the UI would become a way to read
  a key out of any server you administer.
- Web UI: the key field moved into a collapsed **Advanced** disclosure on
  Settings › Server › Reference data, open by default when no key is built in,
  with a summary tag ("own API key set" / orange "API key needed") so a
  collapsed row still says what is inside.

*The pad is high-bit bytes, not a phrase.* The first cut used
`b"VU2CPL-DXCA-cty-obfuscation-pad!"` and `strings` printed it verbatim in the
binary — a signpost standing next to the very bytes it decodes. It is still
only obfuscation: it keeps the key out of `strings`, nothing more, and the
answer to a leak is rotation.

**Verified end to end**, not just compiled: a throwaway server on 127.0.0.1:7593
with its own data dir and no admin key downloaded cty.xml — **402 entities** —
using the built-in key alone, `/api/config/global` reported
`clublog_key_built_in: true` with `clublog_api_key: ""`, the disclosure
collapsed/expanded correctly in the browser with the tag showing, and `strings`
on the release binary found neither the key nor the pad. Full gate green.

**The 1.x macOS app got the same treatment the same day** (its commit is local
and unpushed there) — with a `trap`-based inject/clear in `notarize.sh`, since
SwiftPM has no build-script step to write into a build directory.

**Still to do:** ClubLog ask that a **403 disables further requests
immediately** — they firewall repeat offenders by IP. Neither `refresh_cty`
nor the LoTW/IOTA/FCC jobs treat 403 differently from any other non-200. None
of them retries today so nothing misbehaves, but that is luck, not design.

---

## Session 2026-08-29 (afternoon) — the shell rework

Manoj asked for the UI cleanup the last session had parked, then drove it from
the running screen over roughly twenty rounds. Everything below was found by
LOOKING, which is the same lesson the previous session recorded.

### What changed

**The shell.** Seven tabs became three — **Spots · Alerts · Stats** — plus a
gear that swaps the view for **Settings**, exactly as Meridian does, because
one muscle memory across the two shack apps beats any local improvement. The
Settings rail groups by OWNERSHIP (My station / Server / Access) and carries a
search that matches topic keywords, not just page names: `token` finds
Telegram, `blacklist` finds Reference data, `mqtt` finds Broadcast
destinations.

**Screens that moved.** My ClubLog split three ways: credentials + the alert
ladder to *Settings › My station › ClubLog account*, locator and grey line to
their own page, and the log statistics to *Stats › My ClubLog*. Users and
Blacklist left the tab strip; System dissolved into the Server group. MQTT
folded into Broadcast destinations (one page, two Saves — the UDP rows live in
the TOML, the MQTT rows in the 0600 database). Blacklist folded into Reference
data, because it is the same kind of thing as cty.xml: one server-wide list
every account is subject to.

**The feed grid.** Both tables are `table-layout: fixed` on measured widths, so
a column lands on the same x in every row instead of re-flowing as the stream
runs. Order is now identical on both: Time · DX · DE · Source · Freq · Mode ·
dB · Band · DXCC · Alert · Message/Status. Centred, except kHz and dB (right,
so decimals line up) and Message (left, it is prose).

**The rail.** The five stacked filter rows above the Spots feed moved sideways
into a collapsible rail. Chrome went from **61% of the window to 16%** — the
first spot row was 615px down a 1010px window and is now 143px down 900px.
Collapsed, the spine carries a badge counting active narrowings, because the
house rule is that a narrowing which changes the screen without saying so is
indistinguishable from a feed going quiet. Alerts uses the same rail.

**Contextual help.** Standing prose moved into `?` popovers ported from
Meridian's `HelpTip` — hover to read, click to pin. Empty states, live readings
and per-row tooltips were deliberately left alone.

### Server changes (four Rust files)

- **Grey line ceiling 180 → 360 minutes.** `put_station` refused anything above
  180, so raising it in the UI alone would have produced a save error. Six
  hours is defensible: on the low bands a high-latitude path near the solstices
  stays enhanced that long.
- **`snr_db` on `alerts_sent`** so the Alerts history can carry the same dB
  column the feed does. **Nullable with no default** — every row already in the
  table was written without an SNR, and 0 dB is a real report, so `DEFAULT 0`
  would have put a plausible lie in the history. Added to the existing
  idempotent `ADDED_COLUMNS` list; **ran against production, 199 rows preserved,
  all NULL**.
- **Three-way Telegram spotter gate.** `notify_manual_only` (bool) could only
  ever take skimmers away; `notify_spotter_kind` is `all` / `human` / `skimmer`.
  No migration — the notify config is a JSON blob. The upgrade path is the
  careful part: the new field defaults to EMPTY, which means "predates the
  field", and `notify_config` adopts the old boolean on read; `set_notify_config`
  writes both in step so the adoption can never re-fire over a deliberate
  choice. Unrecognised values **fail open**, and a typo is refused at the API.

### Bugs found by looking, not by tests

- The `?` popover **could not be pinned with a mouse**: the click toggled
  openness, and hovering had already opened it, so every click closed it.
- A popover inherited `white-space: nowrap` from the `.pill` it hung off and
  ran its prose out through the right border. Fixed in `HelpTip` — every feed
  cell is nowrap too, so it would have recurred.
- The Alerts `failed` marker was folded into the 6.5rem Alert cell, which
  clips — so on exactly the rows worth seeing, it was **silently cut off**. It
  has its own `Status` column now, with a header, showing ✓ or Failed either
  way.
- The Alerts table had **no elastic column**: all nine fixed, so nothing could
  give and it truncated on the right under ~1200px.
- Time clipped to `09…` because the column was cut to 4rem without allowing for
  the extra 1rem of left padding the first cell carries.
- `dB` and `Band` were too narrow for their own headers once the sort caret's
  0.75rem was counted.
- A header row was updated while its body edit silently failed to match — **10
  headers against 9 cells**, every column from Freq rightward showing the wrong
  data. Now checked by asserting `colgroup`/`th`/`td` counts agree.
- `.gear` was **already** the shared icon-button class ThemeSwitcher documents
  relying on; the new Settings button had quietly duplicated it.
- Stats landed on the feed charts and the ClubLog data sat behind an unlabelled
  segmented control — read as decoration, so the log statistics looked lost.
  Labelled `Statistics for`, and the choice persists.

### Decisions worth keeping

- **DXCC names are NOT abbreviated.** The question a clipped column has to
  answer is not "does it fit" but "can two entities ever look the same". All
  340 current names were run through `canvas.measureText` at the feed's real
  13.6px system-ui: at **11.5rem** exactly 25 clip and **none collides**. The
  only pair that ever collides is REPUBLIC OF SOUTH AFRICA / SOUTH SUDAN, and
  only at 10.5rem or below. So cty.xml stays the single source of truth and the
  full name is one hover away — a 37-entry override table was drafted and
  binned.
- **Source names capped at 14 characters** in the UDP sources and Cluster nodes
  editors, with a `max` marker at the limit. It is the one column whose widest
  value an operator chooses, so an unbounded name would silently break the fit.

## Session 2026-08-27 (afternoon) — the "2.1 wave"

Read this first: it is the index to everything that changed after the
cutover. Each item has its own section further down with the reasoning; the
M0–M6 progress logs below are history, not current state.

**Features**

| what | where |
|---|---|
| Web GUI restyled to Meridian's design system, light + dark | "Web UI look" |
| Eight alert levels (New ×4 + `?` ×4), band/mode narrowing on display and Telegram independently | "Alert levels 2.1" |
| Station card: DXCC / Challenge / Slots, worked vs confirmed | "DXCC Challenge points" |
| Automatic ClubLog (per-user, daily) + LoTW (server-wide, weekly) re-download | "Automatic ClubLog / LoTW refresh" |
| ClubLog API key moved from per-user to a server setting; cty.xml now admin-only and auto-refreshed | "The ClubLog API key is a SERVER setting" |

**Bugs found and fixed** — all three had been live and silent:

| bug | symptom | section |
|---|---|---|
| DXSpider `\x07` bells defeated the spot parser | db0sue.de proved **Live** and dropped 100% of its spots | "DXSpider bells ate every spot" |
| `systemctl enable --now` never restarts an *active* unit | production ran a **binary older than the one installed** | "Deploy gotcha" |
| `$HOST…` — non-ASCII byte after a variable | `HOST?: unbound variable` under bash 3.2 / C locale | "Shell gotcha" |

**Verified in production, not just built**

- Challenge total **2397 confirmed = exactly what ClubLog reports** for
  VU2CPL (56,815 QSOs, 320/319 DXCC, 4339/4075 slots, 2435 Challenge
  worked). That one match also validates `is_confirmed` and the band table.
- LoTW auto-refresh **fired on its first tick, 13:07**: attempt stamp
  13:07:31, success 13:07:35, file rewritten, 234,734 users live in
  `/api/status`. Next due 2026-09-03.
- Node roster on the Pi is now **VU2OY, N2WQ-2, UberSDR CWskim, Meridian,
  DB0SUE** — five Live. VE7CC was removed by Manoj (deliberate); DB0SUE
  added while chasing the bell bug.

**First third-party install (2026-08-27)** — `adersh@192.168.1.151`, a
remote Pi over VPN, Debian 13 (trixie), deployed with
`deploy/pi-deploy.sh --no-seed`. It self-bootstrapped: its own admin
account via the setup card, its own cty/LoTW downloads. **Nothing of this
station's went to it** — see "Deploying to a Pi that is NOT this shack's",
which is now the rule for any host that is not noderedpi4.

## M0 groundwork

- Cargo workspace (edition 2024): `dxca-core` (spot model + 2 tests),
  `dxca-connect` (doc-only placeholder), `dxca-server` (bin `dxca`).
- Server stub works end to end: loads `config/dxca.toml` (defaults if
  absent, hard error if invalid), serves the embedded Svelte page and
  `GET /api/status`, graceful SIGINT/SIGTERM shutdown. Smoke-tested on
  macOS: status JSON + real Svelte dist served, clean shutdown.
- Web UI: Svelte 5 + Vite + TS under `web-ui/` (pnpm). `pnpm build` →
  `dist/` → embedded by `include_dir` at next cargo build.
  `dxca-server/build.rs` writes a stub `dist/index.html` when absent so
  plain `cargo build` never needs Node (Meridian rule).
- Local gate green: `cargo test --workspace` (4 pass incl. doc-test run),
  `cargo fmt --check`, `clippy --all-targets -D warnings`, web build.
- Pi cross-compile proven **and executed on real hardware**: `just dist` →
  1.5 MB ELF `target/aarch64-unknown-linux-gnu/release/dxca` (aarch64,
  glibc ≥ 2.36) via cargo-zigbuild. Ran on noderedpi4 (Debian 13 Trixie,
  glibc 2.41) 2026-08-26: `/api/status` JSON + embedded UI served, clean
  SIGTERM shutdown. Shack ssh is `vu2cpl@<host>`, key-auth only.

## Plan §11 decisions resolved at M0

1. Repo name: **`dxca`** (this repo, private).
2. Cross-compile: **cargo-zigbuild** (brew-installed with zig; no Docker on
   the Mac, so `cross` was out). Target pinned `aarch64-unknown-linux-gnu.2.36`.
3. Web/telnet bind: default `0.0.0.0` (LAN-service assumption, documented
   in the example config).
4. (Mac-app retirement question stays open until 2.0 is real.)

## Known gotchas

- **This Mac's Rust is Homebrew rustup** with only `cargo`/`rustc` proxies
  symlinked into `/usr/local/bin`. `cargo fmt`, `clippy`, and doc-tests
  need the full proxy set: prefix `PATH="/opt/homebrew/opt/rustup/bin:$PATH"`
  (or symlink the missing proxies like the existing two). Plain builds work
  either way.
- **The ClubLog API key must never become a committed constant.** It is baked
  in by `build.rs` from `DXCA_CLUBLOG_API_KEY` / the git-ignored
  `.clublog-api-key`, into `OUT_DIR` only. ClubLog delete keys they find in a
  Git repository and this repo is public, so folding it back into a source
  literal — obfuscated or not — gets the key revoked for every install in the
  fleet. A build with no key available is *fine*: it behaves as dxca did
  before, with the admin setting one in the web UI.
- **pnpm blocks dependency install scripts**: `web-ui/pnpm-workspace.yaml`
  allow-lists esbuild's postinstall (same pattern as Meridian). Without it,
  `pnpm install` errors with ERR_PNPM_IGNORED_BUILDS.
- `include_dir` embeds whatever `web-ui/dist` held at **compile** time —
  after `pnpm build`, rebuild the server or you serve the old page.
  `just run` sequences this correctly, and so does `install.sh` (which is
  why it always rebuilds in a source tree — see "install.sh did not install
  the web GUI" below).
- Justfile recipe comments must be a single line — `just --list` shows only
  the last comment line above a recipe.
- **The workspace needs rustc ≥ 1.88** (a `Cargo.lock` floor, not ours) and
  distro packages are below it. Declared as `rust-version` in
  `[workspace.package]` and re-checked by `install.sh` (`MIN_RUSTC`) — the
  two move together. See "The rustc floor is 1.88" below.
- **The VPN no longer shadows the shack LAN — SOLVED 2026-08-30.** This sat
  here for two days as unfixable ("disconnect the VPN to talk to the shack"),
  on a diagnosis that was wrong: this Mac is not on `192.168.1.0/24` at all.
  Both tunnels and the shack now run at once. See *Both VPN tunnels and the
  shack LAN, at once (2026-08-30)* below.

## Burn-in log (Mac phase, 2026-08-27 — superseded by the Pi cutover above)

**M2 exit validated live by Manoj**: with the 1.x app stopped, dxca took
over ports 2333/2334/2335 + 7575 on the Mac Mini with the default config.
RUMlog's DX Cluster tab reconnected on its own, its spots table populated
via both paths, and **click-to-fill worked** from the decoders through
dxca's passthrough. MSHV + JTDX ingesting live during validation;
passthrough 180+ datagrams, 0 failures.

Operational state:
- Runs **on the Mac** (decoders send to 127.0.0.1), detached:
  `nohup ~/projects/dxca/target/release/dxca` started from
  `~/projects/dxca` (config-relative paths), log
  `~/Library/Logs/dxca-burnin.log`. Survives Claude sessions, **not** a
  reboot — after a reboot either relaunch it the same way or fall back to
  the 1.x app.
- **The 1.x macOS app must stay closed while dxca runs** (same ports).
  Revert = `pkill -f target/release/dxca`, then launch
  DXClusterAggregator.app.
- Watch it via `http://localhost:7580/api/status` (per-source spot
  counts, per-node honest status, telnet clients, UDP sent/failed) and
  `/api/spots`. The web page itself is still the M0 stub shell — the real
  dashboard is M5.
- **M3 update (2026-08-27):** the burn-in binary now ingests the five
  1.x cluster nodes too (config read from the app's UserDefaults into the
  local `config/dxca.toml` — gitignored). Within a minute of restart:
  VU2OY/N2WQ-2/Meridian/UberSDR-CWskim proven **Live** (Meridian = dxca's
  lifted client logged into meridian's own server), VE7CC sitting
  honest-yellow "Connected, unproven" — the exact 2026-08-24 failure mode
  the honest-status machinery exists for.
- **M4 update (2026-08-27):** the burn-in binary has users+alerts.
  `data/cty.xml` bootstrapped from the 1.x app cache (402 entities
  loaded). **Waiting on Manoj**: create the admin account
  (`POST /api/setup`), then PUT his ClubLog credentials + Telegram
  settings and `POST /api/clublog/refresh` — credentials are his to
  enter, deliberately not migrated from the 1.x UserDefaults by Claude.
- Remaining burn-in gap vs 1.x: no spots-table UI / LoTW markers (M5).
  Aggregation, cluster ingest, RUMlog feeds, and (once the account is
  set up) ClubLog classification + Telegram alerts are at parity.

## M6 progress

**2026-08-27 — v2.0.0 packaged and deployed to both hosts.**

- Version bumped to 2.0.0 (workspace + web-ui).
- `install.sh` (shack-rule compliant: auto-detect macOS/Pi + confirm +
  manual override, never silent): macOS installs a **launchd agent**
  `com.vu2cpl.dxca` (RunAtLoad + KeepAlive, log `~/Library/Logs/dxca.log`);
  Pi installs `/opt/dxca` + a **systemd service** running as `vu2cpl`
  (prebuilt binary preferred, config/data seeded only when absent —
  never clobbers a live install). Templates in `deploy/`.
- `deploy/pi-deploy.sh` — one-command cross-compile + rsync + remote
  install (the plan §9 "one binary + one TOML" deploy).
- **Mac**: the nohup burn-in was replaced by the launchd agent
  (reboot-proof at last); account/db/state untouched.
- **Pi (noderedpi4 = 192.168.1.169)**: dxca v2.0.0 active+enabled under
  systemd in /opt/dxca. State migrated from the Mac (sqlite3 .backup of
  dxca.db → same login works; cty.xml; lotw-users.txt). Its config ships
  the five cluster nodes **disabled** (no dual cluster logins while the
  Mac instance runs) and passthrough aimed at RUMlog on the Mac
  (192.168.10.226:2237 — note the Mac is on the .10 subnet, the Pi on
  .1; routed both ways, verified).

## Cutover — COMPLETE (2026-08-27)

Manoj executed the checklist the same evening: decoders repointed at
192.168.1.169 (all three counting on the Pi), the five nodes enabled via
the System tab (four proven Live immediately, VE7CC honest-yellow as
usual), RUMlog connected to the Pi's telnet server, passthrough to the
Mac's RUMlog clean (0 failures). The Mac launchd agent is stopped; its
plist remains in ~/Library/LaunchAgents (rollback = `./install.sh macos`
or just `launchctl bootstrap`). **Production DXCA = the Pi.** The Mac
databases are now historical; the Pi's /opt/dxca/data/dxca.db is
canonical.

## The decoder cutover (original checklist, kept for rollback reference)

When ready to make the Pi the production aggregator:

1. **Decoders** (all on the Mac): change the UDP server IP from
   `127.0.0.1` to `192.168.1.169`, ports unchanged — MSHV Network Config
   (2333), JTDX Reporting primary UDP (2334), WSJT-X Reporting UDP
   (2335). *(The step used to add "the 2233 ADIF→RUMlog paths stay
   `127.0.0.1` — untouched". Those paths turned out to be unnecessary
   altogether — passthrough carries logged QSOs to RUMlog on 2237. Tick
   MSHV's **Enable Logged QSO** and configure nothing else; see the 1.x
   `docs/UDP-PIPELINE.md` § "Logged QSOs need no second feed".)*
2. **Pi web UI** `http://192.168.1.169:7580` (same login): System tab →
   tick the five nodes' **On** boxes → Apply & save.
3. **Mac**: stop the local instance so it releases the cluster logins:
   `launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/com.vu2cpl.dxca.plist`
   (and delete the plist if permanent).
4. **RUMlogNG**: DX Cluster tab → connect to `192.168.1.169:7575`
   (Data Port 2237 needs no change — the Pi's passthrough already
   targets the Mac).
5. Verify on the Pi dashboard: sources counting, nodes Live,
   click-to-fill in RUMlog.

Rollback = reverse: decoders back to 127.0.0.1, re-bootstrap the Mac
agent (`./install.sh macos`), disable the Pi's nodes (or
`sudo systemctl stop dxca` on the Pi).

## M5 progress

**2026-08-27 (later) — M5 remainder done: web config editing with
hot-apply.**

- `PipelineState` gains swappable internals: `broadcaster()` accessor over
  a RwLock (apply_destinations swaps a fresh UdpBroadcaster — counters
  reset, 1.x `configure` behaviour) and a source-listener registry keyed
  (name, port) with **bind-first** apply: additions bind before anything
  is torn down, so a port clash rejects the whole edit; removals abort
  their tasks (socket drops, port freed).
- `NodeManager::apply` — diff by name + config fingerprint; removed or
  changed clients retire on a blocking task (a supervisor join can block
  up to its connect timeout, never on the async runtime); `start_node` is
  `&self` now (interior mutability).
- `Config` is `Serialize` (scalars declared before array-of-tables —
  TOML emitter requirement), `Config::save` rewrites `config/dxca.toml`
  with a "managed by the web UI" header; hand comments live in the
  example file.
- `GET/PUT /api/config/global` (admin): the three arrays hot-apply +
  persist; unique-name validation; `web_bind`/`telnet_port`/dedupe/ring/
  `data_dir` are returned read-only (file-edit + restart, shown in the
  UI).
- System page: full editors for sources / nodes / destinations with
  add/remove rows, format dropdown, sources-CSV allowlist, unfiltered
  flag, and one **Apply & save** button.
- `tests/config_editing.rs` proves the loop end to end over the real API:
  baseline passthrough → admin edits (source A→B, destination re-pointed,
  node added) → new port live, old port re-bindable, passthrough
  byte-identical at the new destination, old destination silent, node
  dialing, TOML reloaded with the new arrays; duplicate names 400;
  unauthenticated 401.
- Browser-verified on a disposable instance (System page renders all
  editors). Note for future sessions: the embedded browser pane's
  click/type occasionally goes stale right after navigation — re-run
  read_page and retry, or drive fetch() via javascript_tool; Manoj's own
  setup through the real UI worked first time.
- Burn-in restarted on this build. **Manoj created his account** (users:
  1) — ClubLog credentials/refresh + Telegram are his next clicks, and
  node/source editing is now in the System tab.

**2026-08-27 — M5 core complete: the real dashboard, live over a
WebSocket, verified in the browser against a disposable test instance.**

- Server: `/api/stream` WebSocket (per-session spot frames through the
  shared `annotate_spot`, status frames every 5 s), `lotw.rs` port
  (download + 1.x parse/lookup rules; global list in UserService, admin
  `/api/lotw/refresh`, `is_lotw` on every annotated spot),
  `/api/telegram/test`. axum grew the `ws` feature; a registry
  inconsistency forced `futures-util` pinned to 0.3.31 in the lockfile.
- Web UI (Svelte 5; GitHub-dark at M5, restyled to Meridian's design
  system on 2026-08-27 — see "Web UI look" below): session bootstrap → first-run **setup**
  card / **login** card / tabbed main shell. Pages: **Spots** (status
  pills incl. three-state node badges with last-spot age; filters:
  sources, bands, new-only, CQ-only, 60 s hide-duplicates like 1.x
  `displayedSpots`; sortable columns; per-user alert row tints; green
  LoTW dot), **My ClubLog** (credentials + alert levels + refresh with
  counts), **My Alerts** (Telegram + test button), **Users** (admin
  list/create), **System** (server/source/node detail, LoTW refresh).
- Verified live in the embedded browser: setup card on the burn-in;
  on a throwaway instance (scratch data dir, port 7581, since removed) —
  login, six injected spots rendered with LoTW dot on K1JT, and a
  seventh spot appearing at the top **via the WebSocket with no reload**.
- Burn-in restarted on the M5 binary; `data/lotw-users.txt` bootstrapped
  from the 1.x cache (234,467 users). Setup still pending for Manoj —
  the web setup card now replaces the curl commands.
- **M5 remainder** (deliberate scope cut, matching plan §10's "admin
  config editing hot-applies"): sources/nodes/destinations are still
  edited in `config/dxca.toml` + restart; the System page says so. Also
  future polish: spots-table columns for ΔT/low-confidence, per-user
  display-filter persistence.
- Design note: a user with **no matrix** gets no classification at all
  (no alert column, no beacon labels) — deliberate divergence from 1.x,
  which classified everything NEW DXCC against an empty matrix.

## M4 progress

**2026-08-27 — M4 complete: SQLite users, session auth, per-user ClubLog
matrices over the shared stream, Telegram fan-out. Exit criterion proven
in an end-to-end test through the real flows.**

- Deps added (plan §1): rusqlite (bundled), argon2, rand, sha2,
  ureq(+json)/rustls, flate2.
- `dxca-core`: `LogMatrix::build_from_adif` — the exact 1.x
  `ClubLogClient` build loop as a production fn; the local_parity test
  now golden-tests THIS fn against the Swift app's matrix.json.
- `dxca-connect`: `clublog.rs` (cty.php + getadif.php, gzip-by-magic,
  endpoint bases overridable for tests), `telegram.rs` (sendMessage,
  HTML, base overridable). LoTW users list deferred to M5 (display
  marker).
- `dxca-server`:
  - `db.rs` — SQLite (0600): users / sessions / per-user configs
    (clublog + notify JSON) / matrix cache. Secrets at rest plaintext by
    design (plan §5), documented trade-off.
  - `auth.rs` — argon2 PHC hashes; 256-bit tokens, SHA-256-hashed in the
    sessions table, HttpOnly SameSite=Lax cookie, 30-day TTL.
  - `users.rs` — UserService: global resolver (data/cty.xml),
    per-user matrices in memory backed by DB, the 1.x refresh flow,
    per-user classification, Telegram fan-out with the 1.x per-callsign
    cooldown (clamped 5–60 min) and the exact 1.x message format.
  - `api.rs` — full route set (plan §7): /api/setup (first-run admin
    only), login/logout/me, per-user clublog+notify config, refresh,
    admin user management, and /api/spots with per-session
    classification annotations. Composition moved out of main.rs so
    tests drive the real router.
  - Pipeline broadcasts processed spots (`spot_events`); a fan-out task
    classifies per user per spot.
  - Config: `data_dir` (default `data/`), `clublog_base_override` /
    `telegram_base_override` test knobs.
- **Exit test** (`tests/users_alerts.rs`): fake ClubLog (gzipped cty +
  per-user ADIF) and fake Telegram behind real HTTP; two accounts set up
  through the API; both refresh through the real flow; one spot on the
  shared stream → A sees `worked`, B sees `newDXCC`; exactly one
  Telegram ping, to B's bot token; the cooldown suppresses the repeat;
  anonymous /api/spots carries no classification; second /api/setup is
  refused; admin-created accounts don't hijack the admin's session.
- 1.x divergence (deliberate): `maybeNotify` also gated on the display
  filters; server-side notifications gate on levels + cooldown only
  until M5 settles per-user display filters.

## M3 progress

**2026-08-27 — M3 complete: cluster-node ingest with the honest-status
graft, validated against fake nodes in tests and the five real shack
nodes live.**

- `dxca-connect/src/dxcluster/` — the **Meridian lift** (plan §6):
  `client.rs` (sans-I/O ClientSession + supervisor thread) and the
  client half of `wire.rs` (ParsedSpot, classify_line, dx_command),
  diff-minimal with `// DXCA:` markers on every graft:
  - password prompt support (1.x node auth);
  - **honest status**: new `ClientEvent::Proven` fires only on real
    evidence (node prompt, welcome-keyword line, spot/WWV/announce) —
    never on the 30 s login-timeout fallback, which readies the session
    but leaves the pill yellow;
  - 1.x reconnect schedule (10/30/60/120/300 s, last repeats), attempt
    resets **only on proven** — never on bare TCP;
  - watchdog in the connection loop: unproven for `auth_timeout_s`
    (120) → recycle; proven but rx-silent for `silence_timeout_s`
    (15 min) → recycle; both take the normal backoff path;
  - Telnet IAC stripping ported from the 1.x client (N2WQ AR-Cluster
    banners).
- `dxca-server`: `nodes.rs` (NodeManager — per-node status map, event
  consumer thread, `handleClusterSpot`-parity synthetic decodes: message
  `CQ <call>`, SNR/mode scraped from the comment with the 1.x mode-list
  order); pipeline generalized to `PipelineInput::{Datagram,Cluster}`
  with a shared `process_spot` tail; `[[cluster_nodes]]` config;
  per-node status in `/api/status`.
- Tests: 6 session unit tests (password flow, welcome ack,
  timeout-not-proven, no-login feeds, IAC) + the M3 exit-criterion
  integration tests: a fake node proving Live end-to-end into ring +
  telnet, and a **deliberately flaky node** (accepts TCP, never acks)
  staying unproven while the watchdog recycles it with escalating
  attempts.
- Known divergence (documented choice): meridian's spot-line parser
  requires a valid `HHMMZ` time token; the 1.x parser tolerated its
  absence. Lines without one classify as `Line` events and don't count
  as spots/proof. Revisit if a real node exhibits it.

## M2 progress

**2026-08-27 — M2 code complete: the spot path runs end to end.**

- `dxca-core`: `spot.rs` reworked into the faithful `SpotMessage` port
  (dx-callsign extraction with the full `looksLikeCallsign` heuristic,
  CALL-BAND-MODE dedupe key, decode-time→today mapping — mode stays raw,
  `"~"` and all, exactly like 1.x); `format.rs` ports `ClusterFormatter`
  (single-token spotter, pad-or-truncate cells, `HHmmZ`).
- `dxca-connect`: `wsjtx_udp.rs` (tokio source listeners), `broadcast.rs`
  (cluster/wsjtx/passthrough destinations, **v1.8.3 counter semantics** —
  passthrough skipped before bookkeeping; `unfiltered` flag honored),
  `telnet.rs` (1.x-parity server: banner, CRLF fan-out, no login —
  **deliberate deviation from the plan's "lift Meridian's server" line**:
  the 1.x server has no login so parity doesn't need it; Meridian's
  login-capable server comes with per-user telnet feeds in phase 2).
- `dxca-server`: `pipeline.rs` mirrors `ContentView.handleDecode` —
  passthrough-before-parse, per-source dial from Status, 60 s rebroadcast
  dedupe (no-callsign spots bypass dedupe and broadcast as UNKNOWN, 1.x
  parity), spot ring; config grew `[[udp_sources]]` /
  `[[broadcast_destinations]]` with shack-wiring defaults; new
  `/api/spots` + richer `/api/status`; lib target added so integration
  tests can drive the pipeline.
- **End-to-end test** (`dxca-server/tests/spot_path.rs`): real captured
  JTDX Status+Decode vectors sent over real UDP sockets → passthrough
  destination receives both byte-identical, telnet client receives the
  banner and a `DX de JTDX:` line carrying the extracted callsign, ring
  holds the spot with the Status-supplied dial. Passes.
- Display filters (bands/sources/CQ-only/new-only) deliberately absent
  from the broadcast gate until M5 decides whether per-user display
  filters should keep gating the shared feed like 1.x does.

**M2 exit box: CLOSED 2026-08-27** — the live swap-over validated it (see
the Burn-in section above).

## M1 progress

**2026-08-27 — M1 complete: all core-logic ports done, full-chain parity
proven against the Swift app's own artifacts.**

- Ported (`dxca-core/src/`): `adif.rs`, `cty.rs` (with a built-in minimal
  XML scanner — no XML dependency), `dxcc.rs` (resolver + slash-portable
  normalization), `matrix.rs` (serde field names match the Swift Codable
  JSON — 1.x `matrix.json` deserializes as-is), `classify.rs`
  (AlertClassifier + AlertLevel with Swift raw-value serde names, plus an
  `AlertConfig` extracted from ClubLogConfig for the per-user model),
  `bands.rs`, `modes.rs`, `beacons.rs`. 29 unit tests codify the Swift
  behaviours, including the deliberate quirks (ADIF lengths count
  characters; header fields leak into the first record; null/invalid-UTF-8
  QStrings parse as "").
- **Parity test** (`tests/local_parity.rs`, `#[ignore]`d — needs the 1.x
  app's cache, run with `-- --ignored`): parses the real cty.xml (402
  entities, 35,817 rules) and log.adi (56,811 records), rebuilds the
  matrix the way `ClubLogClient` does, and compares against the Swift
  app's own matrix.json. **Exact match on first run**: 320 DXCC statuses
  set-for-set, 26,179 worked calls. Runs in 0.24 s (release).
- Personal log data stays out of the repo — the parity test reads
  `~/Library/Application Support/DXClusterAggregator/` locally.

**2026-08-26 — WSJT-X codec + live-captured vectors.**

- `dxca-core/src/wsjtx.rs`: full parser/builder port of the Swift
  `WSJTXMessageParser`/`WSJTXMessageBuilder`, permissive-parse semantics
  preserved exactly (required fields only for Status: clientId+dialFreq;
  Decode: all but is_new/lowConfidence/offAir; null/invalid-UTF-8 strings
  → ""; unknown type fails the parse). Builder synthesizes Status+Decode
  pairs (deCall `DXCAGGR`, schema 2); `encode_spot` takes `time_ms` from
  the caller — core has no clock.
- `tests/vectors/`: real datagrams captured off the live shack pipeline
  (tcpdump on lo0, 2026-08-26, ~12 min, all three decoders on air):
  8 samples per (decoder, type) for MSHV/JTDX/WSJT-X Heartbeat/Status/
  Decode (+ a type-6 Close from JTDX and WSJT-X), `summary.json` with full
  counts, gzipped source pcap under `raw/`. All schema 2.
- `tests/vectors_roundtrip.rs`: every vector parses; Decodes re-encode
  **byte-identically** (all three decoders); Statuses re-encode as a byte
  prefix modulo null-vs-empty strings. **Emitter quirk worth remembering:
  WSJT-X emits null QStrings (`FFFFFFFF`) for unset fields (dxCall,
  dxGrid…); MSHV and JTDX emit empty ones. The parser collapses both to
  ""** (Swift parity) — the test's `prefix_matches_modulo_null_strings`
  documents this.
- Capture also proved the 1.x passthrough invariant on live traffic:
  1094/1094 datagrams on :2237 byte-identical to a source datagram
  (M2's spec baseline). Extractor: `scripts/extract_vectors.py`.

*Field confirmation of the v2.4.0 spotter work (2026-08-28): Adersh's
alert history grew from 102 rows at v2.4.0 to 109 by the v2.5.0 deploy, and
**all 7 new rows carry a spotter** while the 102 older ones are correctly
empty. The migration's back-fill boundary is exactly where it should be, and
the recording path works in production, not only in tests.*

## Known gotcha: `noderedpi4.local` costs a 5-second mDNS timeout

Noticed 2026-08-29 while timing the new `/api/spot-stats`, and worth
recording because it looks exactly like "DXCA got slow" and is not:

| From | To | Time |
|---|---|---|
| the Mac | `noderedpi4.local` | **5.008 s** |
| the Mac | `192.168.1.169` | 0.004 s |
| the Pi | `127.0.0.1` | 0.001 s |

**Every** endpoint pays it, not just the new one, so it is name resolution
rather than the server — the service answers in a millisecond on the box.
The VPN was **down** at the time, so this is not the subnet clash. It
resolved quickly earlier the same day, so something changed on the network
or in mDNS. Use the IP to avoid it; not investigated further.

## Both VPN tunnels and the shack LAN, at once (2026-08-30)

**The diagnosis this file carried was wrong.** It said the shack and Adersh's
LAN were both `192.168.1.0/24` and therefore irreconcilable. This Mac has a
**single IPv4 address, `192.168.10.226/24` on `en0`**, and the shack's hosts
are *routed* — `route -n get 192.168.1.169` names gateway `192.168.10.1`, not
a connected interface. Nothing ever clashed at the interface level. A tunnel
route simply out-specified the default route.

Two things in the WireGuard configs had to go, and neither is the IPv4 route
one reaches for first:

- **`::/0` in `AllowedIPs`** — each tunnel claimed the entire IPv6 default
  route. Two tunnels cannot both be it, and that is what stopped them
  coexisting.
- **`DNS = 8.8.8.8, …` (Adersh) and `1.1.1.1, …` (vu2wj)** — a `DNS` line
  repoints *system* resolution at a public server for as long as the tunnel
  is up, which is what made `noderedpi4.local` fail. Split routing wants none
  of the remote resolver.

With `AllowedIPs` narrowed to the one host that matters — `192.168.1.151/32`
and `192.168.1.201/32` — and no `DNS` line, every other address in
`192.168.1.0/24` keeps following the default route to the shack.

**The macOS WireGuard app (1.0.16) runs only one tunnel at a time.** Tested:
activating the second deactivates the first, and no preference changes it.
For more than one, run them with `wg-quick`, which gives each its own `utun`.

The configs live in **`/opt/homebrew/etc/wireguard/`** (dir `700`, files
`600`) — one of the three directories `wg-quick` searches by default, so they
go up by bare name, no path:

    sudo wg-quick up Adersh_vu2cpl
    sudo wg-quick up Shaji_vu2wj
    sudo wg-quick up vu2cpl_Ranjith

Deactivate the app's own tunnels first so no peer runs twice. `wg-quick down`
with the same names reverses it. Note `wg-quick` rejects a basename with a
space or over 15 characters — `Shaji _vu2wj.conf` had to be renamed, and
`vu2cpl_Ranjith` fits with one character to spare.

**Verified 2026-08-30, all at once:** adersh `192.168.1.151`, vu2wj
`192.168.1.201`, VU2OY `192.168.220.51` (a third site, PiVPN-hosted, added
the same day), the shack's own `192.168.1.169` and `192.168.1.170`, and the
open internet still leaving directly via `en0` rather than any tunnel. Three
tunnels, two local subnets, one default route, no conflict.

**VU2OY's profile arrived already in the right shape** — `AllowedIPs` a
single `/32` and no `DNS` line — so nothing had to be narrowed. Its LAN is
`192.168.220.0/24`, which collides with nothing here. Its sibling profile
`vu2oy.conf`, a peer of the same server, is a **full tunnel** with its own
DNS: importing that one instead would reproduce the original blackout
exactly.

Two traps met on the way. The exported `.conf` files hold **private keys in
plain text** — keep them somewhere deliberate, not the Desktop, and out of
any shared backup. And `wg-quick` rejects a config whose basename carries a
space or runs past 15 characters, so `Shaji _vu2wj.conf` had to be renamed.

**Gap closed 2026-08-30 — it was a full tunnel.** This entry first said the
original `AllowedIPs` had not been captured, so the cause was inferred. The
untouched exports then turned up in `~/Desktop/vpn profiles/`:
`vu2cpl_wew.conf` and `vu2wj.conf`, matched to the live tunnels by endpoint
hash, both carrying **`AllowedIPs = 0.0.0.0/0, ::0/0`**.

So it was never a subnet route and never a `/24` clash — the tunnels took the
**default route**. Every packet went in, the shack included, which is why
hosts that share no prefix with anything on the tunnel became unreachable.
The original entry in Known gotchas blamed overlapping `192.168.1.0/24`
networks; that was wrong twice over, since this Mac is not on that subnet and
the route in play was `0.0.0.0/0`.

Worth keeping as a diagnostic habit: **read the tunnel config before
theorising about the routing table.** Two days of "the VPN and the shack
cannot coexist" rested on a guess that one line of the config would have
settled.

## Deploy sequence (2026-08-29, standing; network half retired 2026-08-30)

**The networking reason for one-at-a-time is gone** — see the section above;
all three Pis and the Windows box are reachable together now. What stands is
the *human* half of the rule, which was never about routing: the third-party
boxes are not always powered up, and Manoj brings `vu2wj@192.168.1.201`
online himself. So: deploy the shack and the Windows box freely, deploy
`adersh@192.168.1.151` when its tunnel is up, and still **ask before
assuming vu2wj is on**. Never treat "the tunnels are up" as proof a
third-party box is; ping each before deploying to it.

## The installs (2026-08-28; VU2OY added 2026-08-30)

| Host | Account | Notes |
|---|---|---|
| `noderedpi4.local` / `192.168.1.169` | `vu2cpl` | The shack. Seeded deploys; `telnet_interactive = true`. |
| `192.168.1.170` | `manoj` | The shack's Windows box. `win-deploy.sh`, update only. |
| `192.168.1.151` | `adersh` | Third party, over the VPN. `--no-seed` always. |
| `192.168.1.201` (hostname `rpi`) | `vu2wj` | Third party. `--no-seed` always. |
| `192.168.220.51` (hostname `raspberrypi`) | `vu2oy` | **VU2OY (Ranjith).** Third party, over the PiVPN tunnel added 2026-08-30. **Debian 12 bookworm**, self-built. Key auth + NOPASSWD sudo since 2026-08-30. `--no-seed` always. |

**The `~/dxca` source tree on that Pi is Manoj's, not Ranjith's.** An earlier
version of this entry read the tree — on `main`, at `v2.13.0-1-g2928de8`
within the hour of that release — as VU2OY self-building, and drew a
conclusion about `main` being a third party's production input. Wrong:
**Manoj built it there himself, over a RustDesk remote desktop session**,
which is how that box was maintained until 2026-08-30. Ranjith does not build
or deploy; the box is administered from here.

The evidence was consistent with either reading, and the wrong one was
picked. `~/dxca-deploy` was genuinely absent and `known_hosts` had no entry —
both true, both explained by RustDesk rather than by someone else's hands.

One real constraint does survive from that entry: **the aarch64 target must
stay at `.2.36`.** That Pi is on **bookworm, glibc 2.36 exactly** — the floor
`Justfile`'s `aarch64-unknown-linux-gnu.2.36` names, with zero headroom. The
other three Pis are trixie and would not notice the target being raised; this
one would stop starting. Raise it only after moving that host to trixie.

**First deploy from here landed 2026-08-30**, and it retires a manual method:
until now that box was updated **over RustDesk**, a remote desktop session
driven by hand. It is now `deploy/pi-deploy.sh --no-seed
vu2oy@192.168.220.51`, same as the other two third-party Pis.

The first run was deliberately made while he was *already* on v2.13.0, so the
functional outcome was known-good and the thing actually under test was the
path: key auth, NOPASSWD sudo, rsync over the day-old tunnel, and `install.sh`
on **bookworm**. All held. The binary went from 10,290,640 bytes (his native
build) to 7,671,312 (the zigbuild cross-build), which is how you can tell the
swap really happened, and the config md5 was identical before and after.

There is no second party writing `/opt/dxca/dxca` — the `~/dxca` tree there
is Manoj's own RustDesk-era build, so `pi-deploy.sh` simply replaces the
method. The tree can stay as a fallback; nothing has to be told to stop.

The three Pis under `pi-deploy.sh` are aarch64 Debian 13 (trixie). The VPN
hosts were long recorded as un-coexisting with the shack LAN; that was
**wrong and is fixed** as of 2026-08-30 — all three tunnels and the shack run
together, so a deploy no longer needs its own pass. See *Both VPN tunnels and
the shack LAN, at once*.

**VU2WJ's Pi joined the fleet 2026-08-28**, having sat on **v2.1.0** since
its install — nine releases behind, and the only box whose database predated
`alerts_sent` and `blacklist` entirely. The jump to v2.7.0 exercised the
path the migration test was written for and confirmed it: `Db::open` runs
the schema first, so those tables were **created fresh with `spotter`
already in them**, and `migrate()` then found the column present and did
nothing. His account, matrix, cty and LoTW data came through untouched and
both his nodes are Live. Backup at `dxca.db.pre-v2.7.0` on the box.

Its `vu2wj` account had **password-prompting sudo**, which blocks
`pi-deploy.sh` (the installer needs sudo and there is no terminal for the
prompt in a non-interactive run). Manoj granted passwordless sudo via a
validated `/etc/sudoers.d/010-vu2wj-nopasswd` drop-in, matching the other
two boxes. Reversible with `sudo rm` of that file.

## Release convention (2026-08-28, standing)

**A tag is not a release.** Every tagged version gets a published GitHub
release with the Windows zip attached — `deploy/win-bundle.sh`, then
`gh release create <tag> target/win-bundle/dxca-<version>-windows-x64.zip`.
Manoj's instruction, after v2.3.0/v2.3.1/v2.4.0 sat as bare tags: Windows
users have no other route in, since building there needs the MSVC toolchain
the cross-build exists to avoid. Notes should cover everything since the
last *published* release, because tags can outrun releases.

## Open items → next session

### DONE in v2.17.0: the TCI reconnect defect, fixed before the tag

The deferral held: the defect was fixed in the release pass, as decided,
and the release went out with it.

**What was wrong.** `worker` called `pending.clear()` on both the failure
path and a successful re-dial, on the premise that a reconnect means the
server lost the spots we placed. **That premise is wrong for TCI**: a spot
is the panorama's state, not the link's, and outlives the client that
placed it. So any transient drop stranded every mark DXCA had put up —
permanently, and only clearable by hand in ExpertSDR3 — which is the exact
silting-up the per-level lifetimes exist to prevent.

**What it took, beyond deleting two lines.** Keeping `pending` across a
reconnect has two second-order costs the naive fix would have shipped:

* **A busy-spin.** The wait is "the soonest deadline", and after an outage
  every held deadline is already overdue → a zero-length `recv_timeout` →
  a thread spinning at full tilt on the always-on Pi. While disconnected
  the worker now waits for the next *dial* instead, since nothing can be
  sent before then anyway.
* **Dialling a dark radio forever.** A non-empty `pending` is what keeps
  the worker reconnecting, so a radio switched off for a week would be
  dialled every 30s for a week. `PENDING_GRACE` (30 min past due) lets
  those go — by then ExpertSDR3 has almost certainly been restarted and
  the mark is moot — and the worker falls back to blocking on the channel.

**The counter-risk was weighed, not dodged:** if another client re-spotted
the same call during our outage, the delete takes their mark down too.
That is one spot, recoverable by re-spotting, against a panorama that
silts up permanently. Documented in the code at the decision point.

**Tested for real.** `a_reconnect_still_owes_the_deletions_it_could_not_send`
drives a fake TCI server that accepts, takes the spot, drops the link, then
accepts again, and asserts the `SPOT_DELETE` arrives on the new session. It
was **verified to fail** with `pending.clear()` restored (timeout) and pass
without it — a regression test that was never watched fail is not one.
`RECONNECT_AFTER` is `#[cfg(test)]`-shortened to 200 ms, which is what makes
the reconnect path testable inside the gate at all.

Follow-ups 2 and 3 from the original entry (the idle-drain comment
overclaiming, the direct `tungstenite` pin) are **still open** — neither
blocks anything, both are comment/`Cargo.toml` scale.

### DONE (on main, unreleased): four fixes from the first real awards run (2026-09-01)

All four came out of Manoj using the new UI on the Pi, and all four are
worth knowing about beyond the awards feature:

1. **`data.fcc.gov` 403s on `Accept-Encoding: gzip`.** Not the UA, not
   HTTP/2, not the Pi's network — ureq adds that header itself because the
   `gzip` feature is on for ClubLog's gzipped endpoints. Verified against
   the live host: `gzip` and `identity` are BOTH refused, while `*`,
   `deflate`, `br`, `gzip;q=0`, an empty value and no header at all all
   return 200/206. A WAF rule, not content negotiation. `fcc.rs` now sends
   `identity;q=1, *;q=0` (accepted, and the honest ask for a zip), and the
   Pi answers 206 to that exact request.
2. **Help popovers were clipped by the filter rail.** `FilterRail` is a
   scroll container (`overflow-y: auto`) and an absolutely-positioned
   popover cannot escape one, so a tip opened in the rail was cut at 12rem.
   `HelpTip` is **viewport-positioned** now (`position: fixed`, placed in
   `reposition()` from the icon's rect, flipped above when there is no room
   below, re-placed on any ancestor scroll via a capture-phase listener).
   Fixes every tip in both rails and the Settings editors, not just the one
   reported.
3. **A stale award chip kept filtering the Spots feed invisibly.** The chip
   selection persists in `localStorage`; deselecting an award removed the
   chip from the rail but not from the stored set, so it kept narrowing the
   feed — and would have emptied it if it was the only chip. `Dashboard`
   now prunes the stored selection to the levels actually offered, guarded
   on both vocabularies being loaded.
4. **UDP destinations column order**: the wide Sources CSV field sat
   mid-row and pushed Unf / On / ✕ off the right edge; Sources is last now.

### DONE (on main, unreleased): the Awards settings page + declutter (2026-09-01, same day)

The first cut of phases 2–4 folded the award toggles into the ClubLog
page's ladder and let all fourteen levels flood every level list. **Manoj
rejected that shape** — "I wanted the awards as a tab in my awards
settings and user should be able to select which awards he is chasing. I
don't want the spots or alerts to get cluttered" — and this restructure is
the answer, UI-only, no schema change:

* **Settings › My station › Awards** (`AwardSettings.svelte`, new rail
  entry): the DXCC ladder at the top (moved OFF the ClubLog page, which is
  credentials-only again), then one block per award with a **Chasing
  IOTA/WAS/VUCC** tick that reveals its New/`?` pair, per-award data notes,
  and live warnings (FCC table missing → State levels quiet; IOTA
  directory missing → refs unvalidated). "Chasing" is not a stored flag:
  an award is chased exactly when either of its classifier levels is on,
  so a selector and the levels can never disagree. Both pages edit the
  same `clublog_json` row wholesale, the notify_json precedent.
* **The declutter rule**: `AlertLevel::award()` tags each level with its
  award (`null` for the classic eight) and `/api/reference` serves it;
  `web-ui/src/lib/chase.svelte.ts` reads the account's pair flags once and
  every level list filters through it — the Alerts "Ping me for" ladder,
  the Spots Alerts chips, and the Stats Awards card (which now shows only
  chased awards, and nothing at all when none are). Not logged in → nothing
  chased → the app looks exactly as it did before awards existed.

The lesson is the standing one (see the user-memory *scope a UI request as
a UI request*): the request said "tab" and "select which awards", and the
first build answered with architecture instead of the asked-for control.

### DONE (on main, unreleased): IOTA / WAS / VUCC — docs/AWARDS.md phases 2–4

Built 2026-09-01 in one pass on Manoj's "complete the 2-4". The design doc
carries the reasoning; this entry is the implementation map a future
session needs.

**Core (`dxca-core`):**

* `awards.rs` (new) — `US_STATES` (50, DC→MD per WAS rule 6),
  `normalize_state`, `normalize_iota`, `find_iota_ref` (comment scanning),
  and `StateTable`: the 7.9 MB distilled FCC file binary-searched in place
  instead of a ~90 MB HashMap, with the lotw-style slash ladder.
* `Spot` gains `grid` and `iota` (serde-defaulted; **state is looked up,
  not stored** — a deliberate refinement of the design doc, since the FCC
  answer is a server-side fact like `is_lotw`). `grid_from_message` reads
  the trailing locator of an FT8 CQ/exchange; `grid::is_grid` refuses
  `RR73` everywhere, `grid::grid4` folds to the VUCC square.
* `LogMatrix` gains `by_grid` / `by_state` / `by_iota`
  (`AwardStatus` = bands + confirmedBands; serde-defaulted, so every
  stored matrix_json and 1.x matrix.json still loads). Build records
  awards inside the same credit-gated loop (an invalid operation earns no
  grid either); `merge_lotw_confirmed` layers the LoTW QSL report on top,
  additive, never touching by_dxcc. `award_stats()` totals per VUCC band
  plus WAS/IOTA counts and the missing-states list.
* `AlertLevel` grows the six award levels; **FLAGGABLE (now 14) is also
  the tiebreak** — a spot qualifying for several levels flags as the
  rarest, and a level switched off simply stops being a candidate, so
  disabling NEW DXCC lets the same spot flag as New State instead of
  vanishing. `classify_spot(…, AwardRefs)` extends `classify`;
  `Classification.award_ref` names the key that fired. Grid is per band
  and `VUCC_BANDS` only (6M up, no 4M — no US allocation); state and IOTA
  are key-level.

**Connect (`dxca-connect`):** `iota.rs` (groups.json download + directory,
refuses <500 groups), `fcc.rs` (zip download → HD.dat active filter →
EN.dat distill, refuses <100k calls; new `zip` crate, deflate-only),
`lotwreport.rs` (full `lotwreport.adi`, `qso_qsl=yes&qso_qsldetail=yes`;
detects LoTW's HTML-with-HTTP-200 login failure). Always a FULL report:
the matrix rebuilds from scratch, so an incremental pull would shed old
confirmations — the weekly `data/lotw-report-<id>.adi` cache is what keeps
fullness from hammering ARRL, and a failed download falls back to the
stale cache.

**Server:** classify gathers `AwardRefs` (state only when the user's
config could rank it); `synthetic_spot` stops dropping the parsed grid and
scans comments for IOTA; the decode path parses message grids. Six new
`alert_*` classifier flags (the award selector, default off) and six
`notify_*` flags (**default ON** — the classifier pair is the opt-in, so
notify must not be a second gate to find). The phase-1 unconf gate covers
the new `?` levels automatically via `is_unconfirmed`. New admin routes
`/api/iota/refresh` + `/api/fcc/refresh`; `iota_groups`/`fcc_calls` in
status; `award_ref` on annotated spots and in `alerts_sent` (column
migration, `''` backfill); `award_stats` in the station payload. Refresh
scheduler: IOTA monthly by default; **FCC refuses to schedule until the
table exists** — the ~200 MB first pull is always a person's act
(`config/dxca.example.toml` documents both).

**UI:** the 14-level ladder flows everywhere from `/api/reference` (Alerts
+ ClubLog-account FIELD maps extended, level grid now 7×2 pairs); three
new hues (`--alert-iota/state/grid`, GitHub purple/pink/teal) with the
same 58% `?` wash; LoTW credentials on the ClubLog page; IOTA/FCC rows on
Reference data; the IOTA · WAS · VUCC card on Stats; Telegram titles name
the catch ("🟢 New Grid MK83: …"), history rows show the ref.

**Verified:** `just gate` green (275 Rust tests), and a scratch-config
smoke run served 14 levels and loaded the real reference files
(1,178 IOTA groups, 816,973 FCC calls). The §2.6 data checks are resolved
inline in the design doc — ClubLog's export DOES carry GRIDSQUARE (98% of
records), groups.json suffices over fulllist.json, and the FCC numbers
above are from a real distillation.

**Known limits, stated where users meet them:** FCC = license address,
not operating QTH (README, tooltip); IOTA rides cluster comments only;
WAS band/mode endorsements and satellite VUCC deferred; the iota-world
"accepted activations" list (call→ref tagging without a comment mention)
deliberately not consumed — it is a PDF.

### DONE (on main, unreleased): the confirmation-path gate — docs/AWARDS.md phase 1

Two per-account ticks on **Alerts › For the ? levels**, narrowing only the
four `Unconf*` levels — the feature request of 2026-09-01: some operators
simply refuse to QSL, so an unconfirmed entity should only ping for **a new
call that uses LoTW**, a station that can be worked *and* will confirm.

* **The call is new to my log** — `LogMatrix::has_worked_call`, the first
  real consumer of `workedCalls` (1.x carried the field but never read it),
  with the same exact / bare-before-slash / after-slash handling as
  `lotw::is_user`.
* **The call uses LoTW** — the server-wide users list the green markers
  already read; this is the first place it *gates* anything.

The gate is `NotifyUserConfig::passes_unconf_gate`, called from `fan_out`
right after `wants_level`, so it holds Telegram, Flex and TCI alike and
never touches the screen, the telnet feed or MQTT. Both ticks default off
and ride `notify_json` with `#[serde(default)]` — **no migration**, and an
account that has not opted in behaves exactly as before. The `New*` levels
are exempt on purpose: an ATNO is worth working whatever the QSL prospects.
Tests: the gate truth table (`db.rs`), the slash lookup (`matrix.rs`), the
ladder half (`classify.rs`); `just gate` green.

**Deployed to noderedpi4 on 2026-09-01** (`deploy/pi-deploy.sh`, service
restarted clean, all nine nodes back, bundle `index-DVgWuuUL.js` serving
the new rail) — the trial Manoj asked for, gate and TCI together, ahead of
any tag. The release pass above then covers both. Phases 2–4 (VUCC / IOTA
/ WAS) are designed but unbuilt — `docs/AWARDS.md`, including the three
data checks to run before building anything.

### DONE (merged, unreleased): alerts on an ExpertSDR3 panorama (TCI) — PR #1

`crates/dxca-connect/src/tci.rs`, pushed from the same alert fan-out in
`users.rs` that feeds Flex. From VU3ESV, merged to `main` on 2026-09-01 as
`8525e5e`. **The merge carried no version bump** — `Cargo.toml` is still
2.16.0 — so there is no tag and no release; since 2026-09-01 it runs
unreleased on **noderedpi4 only** (the confirmation-path-gate trial deploy
carried it). **Put the version on this heading when it ships.** The Destinations tab list in the
v2.16.0 entry below is left as it is on purpose: that entry records what
v2.16.0 shipped, and TCI was not in it.

**Destinations has a fourth tab** — UDP | MQTT | FlexRadio | TCI. The two
radio tabs are independent in every direction: one, the other, both or
neither, and either without Telegram. Settings are per-account in
`notify_json` under `tci_*`, defaulting off, so a stored row that predates
them reads as off with the Flex fields beside it untouched — there is a test
for exactly that, which is the upgrade risk worth having one for.

**Four things differ from the Flex path**, and each shaped the module:

* **It is a WebSocket, not a raw socket.** `SPOT:...;` written to a plain TCP
  socket is discarded by the server without a word — the same silent-success
  trap `flex.rs`'s header warns about, one layer down. It cost no new package:
  `tungstenite` was already in the tree under axum, so `Cargo.lock` grew by
  one line.
* **`SPOT` has no lifetime argument.** SmartSDR is told how long to keep a
  spot and forgets it itself; TCI is not, so the worker holds each call's
  deadline and sends `SPOT_DELETE:<call>;` when it passes. Same ladder as
  Flex — DXCC 60 min, Band/Mode 15, the rest 1 — but **DXCA enforces it**,
  which means a restart leaves whatever is already on the panorama.
* **`:` `,` `;` are reserved** and truncate the command exactly as a space
  does in SmartSDR's; they become spaces. **Spaces themselves are legal
  here**, the opposite of Flex, so level and entity both fit rather than one
  having to win.
* **`SPOT_CLEAR;` is never sent**, not even to tidy up on connect. The server
  synchronises state across every connected client, so it would wipe the
  spots another logger put there.

The ARGB palette is now **one table with two renderings** — hex for SmartSDR,
decimal for TCI — rather than a second copy that would quietly drift.

**Gate verified on the branch before merging, 2026-09-01.** The four steps
`just gate` runs, run individually with the rustup bin dir on PATH: fmt
clean, clippy clean with warnings denied, `cargo test --workspace` **258
passed / 0 failed** (10 of them new `tci::` tests, two of those standing up a
real WebSocket server), and `pnpm -C web-ui build` clean. On top of the gate,
**both ship targets cross-build** — `x86_64-pc-windows-gnu` and
`aarch64-unknown-linux-gnu.2.36` via `cargo zigbuild`. The new dep carries
`default-features = false`, so no TLS backend is dragged in, which is what
keeps those two clean.

**Known follow-ups — merged with these open (2026-09-01):**

1. **A reconnect abandons every pending deletion.** Both the failure path and
   a successful re-dial call `pending.clear()`, on the premise that the server
   lost the spots. For TCI that premise looks wrong — spots are server-side
   state that survives a client disconnect — so a transient drop with the
   radio still up leaves DXCA's spots on the panorama with nothing left to
   remove them, which is the silting-up the module exists to prevent. The
   counter-risk is real but narrow: re-deleting a call some other logger had
   just re-spotted. The UI and README warn only about a DXCA *restart*, not a
   reconnect. **Deliberately left for the release pass** (Manoj, 2026-09-01) —
   see the NEXT entry above. Of the three it is the one with operational
   consequence, so it goes first in that pass — not rediscovered once the tag
   is being cut.
2. **The idle-cost claim expires after the first alert.** The worker blocks on
   the channel only while there is no link; once connected there is no idle
   disconnect, so it wakes every 250 ms to drain for the life of the process.
   Negligible on a Pi, but the module docs claim more than the code does.
3. **`tungstenite = "0.29"` is pinned directly** while axum is what keeps it
   deduped. An axum bump that moves tungstenite lands two copies in the tree.

**GOTCHA WORTH RECORDING:** a bare `cargo test --workspace` on the Mac dies at
the doctest step — `could not execute process rustdoc … No such file or
directory`. Homebrew's rustup symlinks only the `cargo`/`rustc` proxies into
`/usr/local/bin`; there is no `rustdoc` there. The `Justfile` already prepends
`/opt/homebrew/opt/rustup/bin` for exactly this reason, so **run `just gate`,
never a bare `cargo test`** — a run that stops at the doctests looks like a
much smaller test count than the 258 the workspace actually has.

### DONE: Settings is Sources and Destinations — v2.16.0 (2026-08-30)

Two pages instead of five, each with tabs, mirroring the two ends of the
pipeline:

* **Sources** — UDP | Cluster nodes
* **Destinations** — UDP | MQTT | FlexRadio

FlexRadio moved off its own *My station* entry on Manoj's call: a radio is
somewhere spots go, like a UDP feed or an MQTT topic. It is admin-only now
because that page is, which matches the model — admin is the main user;
guests log in, set ClubLog credentials and choose what their Telegram alerts
on. The settings are still per-account in `notify_json`, so nothing moved
server-side and two operators would each point at their own radio.

**Reused the existing pattern rather than inventing one.** `.segmented` is
already in `app.css`, used by Dashboard, Alerts and Stats with
`role="tablist"` markup and the choice kept in `localStorage`. Stats
remembers its tab for a reason that applies here: a segmented control that
must be found again on every visit gets missed, and someone who came for
their node list would think it had gone.

`Sources.svelte` is a thin wrapper — each tab keeps its own `ConfigGate`,
card and save button, and `UdpSources.svelte` / `ClusterNodes.svelte` are
untouched. Destinations does the same for `Mqtt` and `FlexRadio`.

**Separate save buttons per tab are deliberate, not an oversight**: the UDP
rows live in `config/dxca.toml`, the MQTT rows carry a broker password and
live in the 0600 database, and Flex is in the account's notify row. One
button would have to write three stores and could half-succeed.

### WITHDRAWN: multi-station per-account feeds (2026-08-30)

Built, tested, deployed on noderedpi4 — then removed. `docs/MULTI-STATION.md`
is kept and marked withdrawn, with the findings worth having if it is ever
revisited.

The model it served does not exist here:

> Admin is the main user. Others are all guest users. All sources and local
> network settings to be only with admin. Guests can only login, set their
> ClubLog credentials, and select the spots they want their Telegram to alert
> on.

**No guest owns a source, a node or an output**, so there is nothing to own
per account — one station's feeds, which is what `config/dxca.toml` was
already doing, and what v2.15.1 already implemented.

**The lesson, recorded because it cost a day:** a request to move some menu
items turned into a schema column, a namespacing scheme, an `owner` field on
`Spot`, a second config endpoint and a rewired pipeline, without anyone
deciding a change that size was wanted. Ask what shape the answer should be
before building it — and when the user restates the requirement, check
whether it invalidates the work rather than layering onto it.

Three pieces were kept because they stand alone:

* the `apply_sources` fix freeing a retiring listener that holds a port an
  addition wants — renaming a source while keeping its port was impossible
  without it, failing `EADDRINUSE` against ourselves;
* the telnet first-bytes log, which settled that module's long-standing open
  question: **RUMlog sends nothing at all on connect**, so a callsign prompt
  would need the client to answer one — a separate experiment;
* a `format.rs` test recording that `source_name` reaches the wire as the
  spotter callsign, and that punctuation in it is silently welded rather than
  rejected.

### DONE: the UI cleanup pass (2026-08-29) — see the session entry above

Closed. Everything the note asked for landed: the crowded Spots filter row is
now a collapsible rail, My ClubLog's three unrelated things are three separate
places, and the whole pass was driven in front of the rendered app rather than
from tests — which is again where every defect came from.

### NEXT: ship DXCA's own ClubLog API key (Manoj, 2026-08-29)

**Decided: DXCA embeds its own ClubLog API key and ships it.** This closes the
open question left in *The ClubLog API key is a SERVER setting (2026-08-27)*
below. The reason is not convenience — ClubLog issues API keys to **software
developers, not to ordinary operators**, so the per-install key field is one
almost no user can ever fill. Asking every operator for a key they cannot
obtain is not a configuration step, it is a dead end.

Scope is narrower than it sounds, and the plumbing already exists. The key is
only ever used for `cty.php` (`crates/dxca-connect/src/clublog.rs:39`) — never
for anyone's log, which goes through `getadif.php` with the operator's own
email + app password. And it has been a single server-wide setting since 2.1
(`Db::clublog_api_key`, `db.rs:1051`). This is a fallback default, not a
redesign.

1. **Build-time injection — never a commit.**
   `const BUILT_IN_KEY: Option<&str> = option_env!("DXCA_CLUBLOG_API_KEY");`,
   consulted when the admin setting is empty. **The repo is public:** a
   committed key lives in the history forever and in every clone and fork, and
   it will be found. `strings dxca` still reveals it in a release binary —
   unavoidable and accepted (see "treat any shipped key as public" below); the
   point is only to keep it out of the source.

2. **Stop echoing the key from `/api/config`.** `api.rs:1030` returns
   `clublog_api_key` in plaintext. That is correct while the key is the
   admin's own — they typed it in. It is wrong the moment the key is **ours**,
   because then every admin on every install — adersh's Pi and vu2wj's Pi
   today, whoever else installs it tomorrow — reads our credential out with
   one curl. Return `has_key` plus `key_source: "built-in" | "admin"`, and echo
   back only a key an admin actually set. Read side only — the write path's
   absent-vs-empty `Option<String>` contract (`api.rs:1048`) already behaves.

3. **Bundle a `cty.xml` in the release.** `deploy/win-bundle.sh` ships binary
   + installers + LICENSE only, so a fresh Windows install has no prefix
   database until a download succeeds. With one shared key, a revocation
   becomes a **release** to fix rather than a settings edit — users cannot
   substitute their own, which is the entire premise of this change. A bundled
   file degrades that failure to stale entities instead of a broken install.
   Keep the admin override as the escape hatch for the few who do hold a key.

4. **Set a User-Agent, and tell G7VJR.** Nothing sets one today — ureq's
   default goes out. `DXCA/<version> (+https://github.com/vu2cpl/dxca)` lets
   ClubLog attribute the traffic, which is most of what keeps one key arriving
   from N addresses from reading as abuse. Mail Michael that DXCA is open
   source and the key ships in the binary; the 2026-08-27 note already said to
   ask him first and that still stands. If mirroring cty.xml ourselves ever
   looks attractive (rotation without a release), **ask** — do not assume
   redistribution is permitted.

Request volume is not a worry: weekly per install, and `refresh.rs:118` stamps
the attempt timestamp *before* the call, so a failure waits the full interval
rather than retrying hot.

### DONE: v2.12.2 on all four hosts, in one pass (2026-08-30)

**Nothing outstanding on the fleet.** v2.12.2 is released with the Windows zip
attached, and all four hosts run and report it: noderedpi4 (9 nodes), the
Windows box (2), adersh (4), vu2wj (2). Both third-party `config/dxca.toml`
md5s are byte-identical before and after, which is what proves `--no-seed`
did its job. The redeploy this section used to ask for is done — no host
reports a version it is not running.

**This was the first deploy that never touched a tunnel switch**, and the
first real test of the fix in *Both VPN tunnels and the shack LAN, at once*:
shack, Windows and both third-party Pis were reachable throughout, in one
sitting, with no disconnect-reconnect and no prompting. The recipe held —
`dxca.db` copied to `dxca.db.pre-v2.12.2` on each third-party box first,
`--no-seed` on both, md5 verified after — and, unlike every prior VPN deploy
recorded here, **no transfer failed and nothing needed retrying**.

**Ping is not a liveness test for the Windows box.** `192.168.1.170` does not
answer ICMP (Windows blocks it), and a first check read as "no answer" while
the host was up and serving. Probe a port instead — 22, 7580 and 7575 were
all open. Do not conclude that box is down from a failed ping.

### DONE: the gate passes, for the first time in this project (2026-08-30)

`just gate` is green end to end — fmt, clippy with warnings denied, **204**
tests, web build (`1efe388`). Everything below is closed; it is kept because
the *reason* it went unnoticed is the reusable part.

**It was never toolchain drift.** An earlier version of this entry said
`channel = "stable"` had moved to 1.96 and broken the gate. Wrong: rustc
**1.96.1 was installed 2026-07-08** and this repo's **first commit is
2026-08-26**, so dxca has only ever been built on one compiler. The failures
were there from the beginning.

**Why nobody saw them, which is the lesson.** `cargo fmt` and `cargo clippy`
need the Homebrew rustup PATH prefix that Known gotchas already documents.
Without it they are not subcommands at all, so `just gate` died on its first
line and what actually got run was `cargo test`, which passed. **A gate that
cannot run is indistinguishable from a gate that passes.** The Justfile now
exports that PATH itself, so the gate runs wherever it is invoked from.

What the reformat turned up, none of it cosmetic:

- **A test that had never run.** `db.rs`'s
  `sent_alerts_keep_failures_and_stay_bounded_per_user` had no `#[test]`
  attribute. It passes now that it runs, so nothing was broken behind it —
  but nothing was being checked either.
- **Two tests that ran twice.** `db.rs:1162` and `:1297` each carried
  `#[test]` before *and* after their doc comment. That is why the total moves
  from 205 to 204 while a test is added: −2 duplicates, +1 revival. The 205
  this file used to quote was inflated.
- `solar.rs` had a round-to-midnight that did nothing (its constant is
  already an exact midnight) and an `i64 -> i64` cast.
- Two fake telnet nodes discarded a read count implicitly; now explicit.
- `telegram.rs` keeps `result_large_err` with a written reason — the Result
  never leaves the function, so boxing would buy stack nothing is short of.

The toolchain is pinned to `1.96.1` (`rust-toolchain.toml`) so the ground
stops shifting under the fleet. Bumping it is now a deliberate act: raise the
pin, run `just gate`, fix what the new rustfmt and clippy think, same commit.

The published v2.12.2 release notes originally carried the wrong
toolchain-drift explanation; corrected 2026-08-30, with a note that the fixes
land after the tag and touch no shipped code.

### DONE: Flex settings moved, page renamed, lifetimes per level — v2.15.1

Three asks in one pass.

**Its own page under My station, not with the spot outputs.** Manoj asked to
move it to the destinations page; the finding that changed the answer is that
the **Server group is admin-only** (`GROUPS.filter((g) => !g.admin ||
isAdmin)`) while the Flex config is **per-account**, living in that user's
`notify_json`. Filing it there would have hidden it from any non-admin whose
radio it actually is. Every install currently has one admin user, so nothing
would have broken today — which is exactly why it needed saying now.

**"Broadcast destinations" → "Spot outputs."** Nothing there broadcasts; they
are unicast UDP sends and MQTT publishes. "Outputs" pairs with the UDP
sources and cluster nodes above it, which are where spots come *in*. Renamed
in the nav, in the page heading and in its HelpTip — the heading was still
saying the old name after the nav changed, which would have shipped as a
visible mismatch.

**Per-level lifetimes, adjustable.** New DXCC 60 min, New Band/Mode 15, all
else 1 — the last being the one that matters: New Slot and the four `?`
levels are most of the alert traffic, and at twenty minutes they paint the
whole band inside an hour, burying the red mark the feature exists to show.
Built hard-coded first, then made adjustable on Manoj's follow-up; 0 in any
field means the default beside it, as `flex_port` already did.

**Three pages now write one `notifications` row** — Alerts, Telegram and
FlexRadio. All three load the whole object and spread it back, and it was
verified in the browser both ways: save on FlexRadio, check the DB, save from
Telegram, check the flex fields survived.

**Two labels wrapped again** — "New Band / Mode (min)" and "Everything else
(min)" overflowed the settings grid, the same defect as the health fields
yesterday. Shortened to "Band/Mode (min)" and "Others (min)". The column
takes about 15 characters; anything longer wraps and drops the `?` icon to a
second line. **Write that down: 15 characters is the budget.**

**MISTAKE WORTH RECORDING: a local run dialled the production cluster nodes.**
Restarting the look-only instance as `cd DIR && nohup BIN &` did *not* carry
the working directory — the process came up with cwd
`/Users/manoj/projects/dxca`, read the repo's `config/dxca.toml`, bound 7580
and connected to VE7CC and VU2OY under this station's `login_call`, which is
precisely the session fight HANDOVER warns about. Killed within ~30 seconds;
noderedpi4 never lost a node (9/9 throughout). **Use `cd DIR && (BIN > log
2>&1 &)` and then verify** — check the startup line says the sandbox port and
`nodes []`, and check `lsof -a -p PID -d cwd`. A run.log in the right
directory does not prove the process is in it.

### DONE: alerts on the FlexRadio panadapter — v2.15.0 (2026-08-30)

`crates/dxca-connect/src/flex.rs`, pushed from the alert fan-out in
`users.rs`. Asked for as *"a new format in broadcast destination for
flexradio to port 4992"* — and the useful part was that it cannot be one.

**Why not a `Format`.** Every broadcast format is a UDP datagram to an
address; 4992 is a TCP session with sequenced `C<n>|` commands and `R<n>|`
replies. A `Format::Flex` would have been a configuration row that looks
right and silently does nothing. Same conclusion MQTT reached, for the same
reason — a sibling module, its own list.

**The command was ported from Manoj's working Node-RED flow, not from the
API docs**, so the field set is one already proven against his radio:
`rx_freq`, `callsign`, `mode`, `comment`, `spotter_callsign`, `timestamp`,
`color`, `priority=2`, `lifetime_seconds`, `source`. Keep that provenance in
mind before "improving" the field list.

**It is alerts, not the feed — and that is the whole point.** His flow keyed
on `msg.alert` and coloured by level, which is exactly the per-user filtered
output that was previously called impossible without new work. The alert
level comes from the account's ClubLog matrix, which Aether cannot see, so
this is the *only* route by which a panadapter shows "New DXCC" rather than
"a spot". Hooked into the fan-out beside Telegram so every existing
narrowing — levels, bands, modes, spotter kind, band mask, cooldown —
applies unchanged.

**`fan_out`'s gate had to change.** It bailed on `!telegram_enabled`, which
would have made a Flex-only account silent. It now asks whether *any* sink
wants the alert.

**Three implementation points that are not obvious and will bite whoever
edits this:**

1. **No value may contain a space.** The command is space-delimited
   `key=value`; one space inside a comment truncates it and the radio parses
   the remainder as garbage. `sanitize()` is load-bearing, and a test walks
   every field asserting it still contains `=`.
2. **The socket must be drained.** 4992 is bidirectional and the radio
   streams status messages from the moment you connect. A writer that never
   reads fills its receive buffer, the window closes, and the radio blocks on
   us. Every connection carries a reader thread that discards.
3. **Colours come from the dashboard's dark palette**, with the four `?`
   levels precomputed as the stylesheet's 58% `color-mix` toward muted — so
   the radio and the screen agree. Manoj's flow had three colours; the other
   five are new.

**Decision (Manoj): Aether's cluster feed comes off.** With DXCA pushing
alerts directly and Aether pushing every cluster spot, each alert would land
twice. The panadapter now shows alerts only — sparse and high-signal. Aether
stays the SmartSDR client, just not the spot source.

**Checked against the real radio, twice, and both checks changed the code.**
A passive connect showed the greeting — `V1.4.0.0`, a handle, `M…|Client
connected from IP`, then `S<handle>|radio slices=3 panadapters=3` — proving
no slice is claimed (that 3 was Aether's) and measuring the status stream at
**1,424 bytes in six idle seconds**, which is the drain justified rather than
theorised. Then `C1|client program DXCA` was tried and **refused**:
`R1|10000002|unknown client program`. SmartSDR validates the name against a
list it knows, so the handshake stays empty — which is what the Node-RED flow
always did, now for a recorded reason. A test asserts the first line on the
wire contains no `client` command at all, so `client gui` cannot creep in.

The dummy spot went through on the first try: `R7001|0|4328` — code 0,
spot index 4328, P5ABC on 28.095 in red on the panadapter.

**Comments prefer the entity.** `NEW DXCC DPRK (NORTH KOREA)` clipped to 20
read `NEW_DXCC_DPRK_(NORTH` — the level twice (the colour says it already)
and the entity cut mid-word. `flex::comment_for` now drops the label when
both will not fit, giving `DPRK_(NORTH_KOREA)` whole.

**Tested against a real socket, not just as a string.** `TcpListener` on an
ephemeral port, a stand-in radio that talks first and never stops, asserting
connect, write, session reuse across two spots, and the sequence advancing —
plus a dead-radio test proving it counts the failure instead of wedging.

**The UI defect this time: the port and lifetime fields rendered `0`.** The
server stores 0 to mean "use the default", which is right on the wire and
useless on screen — a port field reading 0 says nothing about what will be
dialled. The load path now fills 4992/20 in for display. Found by looking,
again; the third such defect in one day that no test could have caught.

### DONE: health alerts — v2.14.0 (2026-08-30)

Asked for after vu2wj sat dead long enough that it was noticed by accident.
`crates/dxca-server/src/health.rs`, spawned from `main.rs` beside
`refresh::spawn`, plus two fields on the Telegram settings page.

**The scope argument came first, and it is the useful part.** Manoj's own
framing — *"i dont think if internet itself is down at the site, u can do
anything? i dont want a central monitoring for all hosts... too much"* — is
right, and settles the design. A dead host cannot report itself; only an
external observer could, and a five-host fleet does not warrant one. So each
install watches *itself* and tells *its own* operator through the Telegram it
already has. Decentralised, no new service, nothing polling anyone else.

**Be honest about the hole:** Telegram needs the internet, so a connectivity
failure silences the alert about the connectivity failure. **This would not
have caught vu2wj**, and saying so was part of the offer.

Two conditions, both `0 = off` and off by default:

- **Feed quiet** — nothing from any source for N minutes.
- **Node down** — one node *disconnected* for N minutes.

**Node health keys on the connection, never on traffic.** "Connected, zero
spots" is a normal state — Hamalert and KST2Mac sit `Live` with
`spot_count: 0` for hours on noderedpi4 — so alerting on silence would cry
wolf every quiet afternoon.

**The feed clock is a counter diff, not a timestamp on the hot path.**
`process_spot` runs for every decode on every band; summing `source_counts`
and the nodes' `spot_count` once a minute answers "the total has not moved
since T" without touching it.

**Edge-triggered, with recoveries.** One message in, one out. A monitor that
repeats every tick trains its reader to ignore it; one silent on recovery
sends them to go and look, which is what it was meant to save. State lives in
the task, so a restart re-announces a still-quiet feed a threshold later —
the right way round after a restart.

**Two traps found by looking rather than by tests, which is the recurring
lesson here:**

1. **The labels wrapped.** "No spots for (min)" and "Node down for (min)"
   overflowed the settings grid's label column, dropping each `?` onto a
   second line and misaligning the inputs, while the Cooldown row above
   stayed clean. Shortened to "No spots (min)" / "Node down (min)". Nothing
   in the gate could have caught that — it was a screenshot.
2. **`var(--fg-muted)` does not exist**; the token is `var(--muted)`. A
   missing custom property fails silently to inherited colour, so the build
   passed and the heading was simply the wrong colour.

**The cross-page write hazard was checked, both ways.** Alerts and Telegram
both PUT the whole `notifications` object, and the new fields carry
`#[serde(default)]` — so a page sending a partial would silently zero them.
Both load the full object and spread it back, and it was verified in the
browser: set 30/10 on Telegram, saved from **Alerts**, re-read the database,
still 30/10.

**Looked at in a browser before shipping** — on a throwaway instance in the
scratchpad (own empty database, `web_bind` 127.0.0.1:7690, no cluster nodes
so it could not fight the Pi's sessions under this station's `login_call`).
That is the pattern to reuse; `config/dxca.toml` as it stands is not safe for
a local look-only run.

### DONE: the panadapter feed — Aether on the telnet server (2026-08-30)

**The 2026-08-28 TODO is answered, and the answer was not MQTT.** That entry
ended with DXCA's publish side proven and *nothing subscribing*, because
neither Flex nor Aether reads MQTT natively and no Node-RED bridge was ever
written. Manoj asked whether a **broadcast destination** could do it instead;
it cannot, and the reason is worth keeping.

**All three UDP formats are the wrong shape for a Flex.** `cluster` is a
`DX de …` line in a *UDP datagram* — the trap here, because it looks right
while anything expecting a DX cluster wants a TCP telnet session; `wsjtx` is
synthesized WSJT-X binary packets; `passthrough` is a raw decoder relay.

**What worked, with no code at all: Aether pointed at the telnet cluster
server on 7575.** Aether is the DX-cluster consumer, and DXCA has been a
cluster server since M3. In trial as of 2026-08-30.

Two things to know about that feed, from `pipeline.rs:346-368`:

- It is **dedupe-filtered but not user-filtered**: first spot per
  CALL-BAND-MODE per window reaches telnet. A spot with no callsign has no
  dedupe key and is broadcast anyway, as UNKNOWN.
- **It carries every spot, not alerts.** Alert levels are per-user — they
  depend on that account's ClubLog matrix — while telnet, UDP and MQTT all
  carry the server-wide feed. `LOGIN` is for passing cluster commands
  upstream, not for filtering.

**So "only New DXCC on the panadapter" is not expressible today**, and it is
the obvious next ask if the trial shows the full feed is too much. It would
need a per-user filtered output, which is a real feature and not a
destination setting — do not bend `unfiltered`/`allowed_sources` into it.

**CORRECTION: Aether does read MQTT.** The 2026-08-28 entry concluded
"neither is known to read MQTT natively" and wrote the route off on that
basis. Wrong — Manoj had configured Aether's MQTT for spots himself, and
`lsof` shows AetherSDR holding three connections at once:
`192.168.1.148:4992` (the Flex API), `192.168.1.169:7575` (DXCA's telnet
server) and `192.168.1.169:1883` (the broker).

**Why it was silent, and it is neither app's fault.** The broker ACL has:

```
# AetherSDR panadapter display — read-only, humanized aether/* tree only
user display
topic read aether/#
```

DXCA publishes to `shack/dxca/spots/{json,cluster}`. So a subscription to
`shack/…` is **denied by the ACL** — and mosquitto's refusal is invisible to
most clients, giving a healthy connection and no messages — while a
subscription to `aether/…` is allowed but reads an **empty tree**, because
the "humanized" republisher the comment assumes was never written. Either
way: connected, authenticated, silent.

**Nothing consumes the MQTT spots at all.** The active flows file has 57 MQTT
nodes and **zero** `shack/dxca` references; adersh and vu2oy have no MQTT
destination configured; only noderedpi4 has one, enabled, feeding nobody.

**Decision (Manoj, 2026-08-30): retire MQTT, keep the telnet cluster.** He is
removing the destination by hand — nothing here was changed for it. If it
goes, the broker's `display` user and its `aether/#` ACL entry become dead
config and are worth tidying in the same pass.

**Gotcha that nearly produced a false negative:** this Node-RED runs in
**projects mode**, so the live flows are
`~/.node-red/projects/vu2cpl-shack/flows.json` (712 KB), *not*
`~/.node-red/flows.json`. A glob over the latter answers about the wrong
file, and an unreadable path greps as cleanly as an absent string — check
the byte count before believing a "nothing found".

### DONE: uncredited contacts are named at refresh time (2026-08-30)

Asked for after the whitelist fix raised the obvious next question. VU24DX's
`ZL8AC` was found by hand — resolve 22,184 worked calls, cross-reference 59
whitelisted entities, cross-reference cty.xml — and Manoj's reply to the
answer was that **no such QSO exists in his log**. Which is a second question
the matrix cannot answer, because it stores no dates.

Both credit rules were silent by construction: the QSO simply vanishes from
the totals, and the only symptom is a number one off ClubLog's. So the build
now reports what it dropped.

- `LogMatrix::build_from_adif_reporting` returns
  `(matrix, count, Vec<UncreditedContact>)`; `build_from_adif` delegates to it
  and discards the third, so no caller had to change. A test asserts the two
  stay identical, because they would drift silently otherwise.
- `UncreditedContact` carries the **raw ADIF strings** — call, QSO_DATE,
  TIME_ON, band, the log's own mode (not the DATA bucket) — so a printed line
  can be searched for verbatim in the operator's ADIF.
- `refresh_user` prints them, capped at 50 with a "... and N more not listed"
  line. A cap that hides its own truncation would read as "that was all of
  them".
- Nothing is printed when there is nothing to report, or the line becomes
  noise every refresh and stops being read.

Against VU2CPL's real log it reports three, all `V55DX` in 2016 — invalid
from 2016-01-01 onwards — and all three were already uncounted, because
ClubLog exports them with no DXCC field. Totals unchanged, but now visible.

**It ran, and the answer is `ZL8AC 20220927 011545Z 40M FT8` — 27 September
2022 at 0115Z.** Manoj says no such QSO exists in his log; the date is what he
now has to check it against. DXCA read it out of the ADIF ClubLog served, and
cannot have put it there (two endpoints, both reads).

**VU24DX has 18 uncredited contacts, and all 18 stand up to inspection:**

- **6 × EP3FS** — a ClubLog `<exception>` mapping the call to **adif 0**, a
  non-DX operation, from 2016-03-13 onwards, and on the invalid list too.
- **11 whitelist rejections, 9 of them Iranian.** Iran is whitelisted from
  2019-01-01 precisely because of its history of unlicensed operations, and
  EP3GMR / EP3IXF / EP5JMS / EP2ACH / EP5HD have no exception record at all.
  EP7AAD *is* listed — but only from 2024-06-16, and the QSO is 2022.
- **T31TTT 2023-09-22** — listed for Central Kiribati only for
  2026-03-27..06-03. A different, uncredited use of the call two and a half
  years earlier, not a boundary artifact.
- **ZL8AC**, the original.

**The validation that matters: 313 worked and 307 confirmed, both matching
ClubLog exactly, and `slots_confirmed` unchanged at 2,647 — not one confirmed
QSO was rejected.** That is the false-rejection tell from the Wake Island
case, and it reads clean. `slots_worked` fell 2,973 → 2,971: Kermadec's
40M-DATA (the entity went with it) and Central Kiribati's 10M-DATA (the 2023
T31TTT). Iran lost no slot — every EP rejection sat on a band/mode already
held by a whitelisted call.

**What was ruled out first, and the method is worth keeping.** Before adding
anything: `zl8ac` is at line 22,079 of the stored `workedCalls` (an exact
match, not an inference); of 22,184 calls exactly two look unusual and both
are real special-event calls, so a misaligned ADIF parse is not the source;
DXCA reaches ClubLog at two endpoints only, `cty.php` GET and `getadif.php`
POST-to-download, so it has no way to have written the QSO; and the
configured logbook is `VU24DX`, the same one being searched. The callsign was
in the file ClubLog served. Where it came from *before* that is a question
only the ADIF can answer — hence this feature.

### DONE: win-deploy.sh follows %SystemDrive%, not a hardcoded C: (2026-08-30)

Found by Manoj asking "what if the Windows install is in D:\?" — a question,
not a bug report, and the answer was that the two Windows scripts disagreed.

`install-dxca.cmd` has always used `INSTALLDIR=%SystemDrive%\DXCA`, so on a
machine whose Windows boots from D: it installs to `D:\DXCA` and registers the
scheduled task with that path. `win-deploy.sh` hardcoded `C:\DXCA`. On such a
box it would have failed at its first precondition with *"no dxca.exe in
C:\DXCA — run install-dxca.cmd for a first install"*: telling the operator
there is no install when there is one, on the other drive.

Safe, at least — it fails before stopping anything, so nothing goes down. But
the message points at the wrong remedy, and running install-dxca.cmd on that
advice would have been harmless only by luck.

Fixed by asking the box (`echo %SystemDrive%`) instead of assuming, and
building both the Windows and the SFTP form of the path from the answer. The
**shell check moved to the top** as part of this: `%SystemDrive%` expands only
under cmd, so probing it before proving the shell is cmd would silently yield
the literal string. A non-drive answer falls back to `C:` rather than building
a path out of garbage — and the dxca.exe check downstream turns a wrong guess
into a clear error rather than damage.

Verified against the real box: prints `Install directory: C:\DXCA` and
deploys as before. Paths corrected in `README.md` and `README-WINDOWS.txt`,
the latter with a note that a different drive is **found**, never **chosen** —
there is no prompt, because one fixed location is what stops upgrades asking.

### DONE: the entity whitelist — the *actual* 314 vs 313 (2026-08-30)

**The invalid-operations fix below was correct, and was not the cause.** It
shipped as v2.13.1, deployed to all five hosts, VU24DX refreshed — and still
read 314. Mount Athos was a wrong diagnosis: `SV2RSG/A` *is* on ClubLog's
whitelist, and VU24DX's QSOs with it fall outside all three rejected windows.
Manoj said so before the evidence did ("i dont think its mount athos").

**The real mechanism is a third list: `<whitelist>` on the entity.** 59 rare
entities carry it. For those, ClubLog credits *only* callsigns it lists as
exceptions; a bare prefix match earns nothing. VU24DX has one `ZL8AC` QSO on
40m — `ZL8` resolves to Kermadec, and **ZL8AC appears nowhere in cty.xml**,
not as an exception and not as an invalid operation. That single QSO was the
whole difference.

**How it was isolated, and the method is the reusable part.** Confirmed
matched at 307, so the extra entity had to be one of the seven
worked-but-unconfirmed ones. Resolving VU24DX's `workedCalls` locally gave the
single callsign behind each — T32TT, RI1FJL, ZL8AC, SV2RSG/A, S01A/S01WS,
T33T. Cross-referencing those against `<whitelist>` left exactly one that
ClubLog would not credit. No ClubLog credentials, no ADIF: the stored matrix
plus cty.xml was enough.

**A test on the real cty.xml is what separated code from data.** When the
deploy changed nothing, `the_real_cty_flags_a_known_invalid_operation` proved
the invalid-ops path worked end to end against the live 10 MB file — so the
fault was the hypothesis, not the implementation. Write that test *before*
theorising about a second bug.

Implementation notes:

- `DxccEntity` gained `whitelist` and `whitelist_start_unix`. Ten entities are
  whitelisted only from a date (Turkmenistan 2007, Iran 2019, the Pacific
  islands from the 1978 rule change); before it, ordinary calls counted.
- The resolver keeps whitelisted entities' exact rules in a **separate** map
  from `exact`, because that one is filtered to rules active *now* and a
  whitelist is a historical record — ZL8X counted in November 2010 and counts
  still.
- **Both sides of the slash are matched.** `K9HEI/KH9` in a log is
  `KH9/K9HEI` in cty.xml. Literal matching rejected a *confirmed* Wake Island
  QSO from VU2CPL's log — and a rejected contact carrying a QSL is the tell
  for a false rejection, because a QSL match means the operation was real.
  With the swap, VU2CPL's log loses nothing: 320 entities, 26,179 worked
  calls, 4,336 slots, unchanged.
- The Swift parity test now clears `whitelist` as well as
  `invalid_operations` to reproduce 1.x.

**The live classifier deliberately does NOT apply the whitelist.** It runs on
spots, where cty.xml's lag cuts the other way: a genuine new DXpedition is not
listed yet, and suppressing its New DXCC alert would hide the rarest catch of
the year. So a ZL8 spot still alerts; only the *log* side refuses the credit.
The two now agree by construction — the matrix says Kermadec is unworked, so
a Kermadec spot alerts as New DXCC, which is exactly right.

**Known consequence, documented in the README:** cty.xml lags real
operations, so a DXpedition ClubLog has not listed yet reads as un-worked
until the next refresh. Correct — ClubLog will not credit it either — but it
is the likely explanation next time something rare "won't clear".

**Predicted before deploying, on Manoj's instruction, and worth repeating as
a method:** the whitelist was simulated against VU24DX's stored matrix
(entities + `workedCalls`) and cty.xml alone — 35 whitelisted entities in that
log, 34 with an accepted call, one without. Predicted 314 → 313 with no
collateral loss, before a binary went anywhere near the Pi.

### DONE: LoTW marker in Telegram alerts (2026-08-30)

The Spots table has marked LoTW uploaders with a green `●` since M5; Telegram
did not. It does now — an **asterisk after the callsign**:

```
🔴 NEW DXCC: 3Y0J*
```

**Asked for as a green dot, delivered as an asterisk, on Manoj's own
correction mid-build** — and the constraint is real either way: Telegram's
HTML supports `<b>`, `<i>`, `<a>` and friends but **no colour**, so a `●`
arrives uncoloured and reads as punctuation. The only green dot Telegram will
render is the emoji 🟢, which beside a callsign competes with the level emoji
that carries the actual urgency. An asterisk is the footnote mark this
already is.

`alert_html` gained an `is_lotw` argument, fed from `is_lotw_user(&call)` at
the fan-out site. The mark is concatenated onto the callsign *before*
`escape_html`, not onto escaped output, and it rides on the call rather than
the label so it is present at every alert level. Two tests: one asserts the
marked and unmarked messages differ in nothing else, one walks four levels.

Not done, deliberately: the **Alerts history** table does not show the mark.
It stores what was sent, and the LoTW list is mutable — a call that was not
an uploader in March may be one now, so a historical row rendering today's
answer would be quietly wrong. Add it only with the flag stored per row.

### DONE: ClubLog's invalid-operations list is honoured (2026-08-30)

**Found by a question, not by a test.** VU24DX's Stats tab read **314 DXCC
worked** while the ClubLog DX Dashboard embedded three inches below it read
**313**, on the same 65,908 QSOs, with confirmed agreeing exactly at 307.

The cause: `cty.rs` parsed three sections of cty.xml — `<entities>`,
`<exceptions>`, `<prefixes>` — and never `<invalid_operations>`, which
carries **2,838 entries** naming callsigns (many with a date window) whose
QSOs do not count for DXCC. ClubLog resolves such a contact to an entity but
refuses to credit it; DXCA credited it. 1.x had the same gap, so this is the
port's first deliberate divergence from the Swift app.

**The entity was Mount Athos.** VU24DX's log reaches it only through
`SV2RSG/A`, an operation ClubLog rejects in three windows (2020-05-01/06,
2021-12-09 13:40 onwards, 2024-09-05/09). Confirmed matched at 307 because
nothing there is confirmed — which is also how the entity was isolated: of
the seven worked-but-unconfirmed entities in that log, it is the only one
whose calls appear on the invalid list.

**Diagnosing it needed no ClubLog credentials.** `LogMatrix.worked_calls` is
in the stored matrix, so intersecting it with the invalid list on the host
itself named the 37 flagged contacts in that log directly. Worth remembering:
the matrix answers more questions than the stats endpoints expose.

What changed:

- `cty::InvalidOperation` + `CtyData::invalid_operations`, parsed from the
  section. `covers()` holds the window logic: no window at all means always
  invalid; a *windowed* entry needs a QSO time and does not match without one
  — never discard a contact we cannot place.
- `DxccResolver::load` now takes the whole `CtyData` instead of
  `(entities, rules, now)`. Deliberate: passing the pieces let a caller drop
  the invalid list silently, and the symptom of that is not a compile error
  but a wrong DXCC total.
- `is_invalid_operation(call, at_unix)` matches the **raw** call, never the
  portable-normalised one — `SV2RSG/A` normalises to `SV2RSG`, a different
  and valid station, which would both miss the flagged call and smear the
  flag onto an innocent one.
- `Record::qso_datetime_unix()` — QSO_DATE + TIME_ON.
- `build_from_adif` skips flagged contacts before resolution *and* before
  `worked_calls`, so a station worked only invalidly still alerts as new.
  The returned QSO count is untouched: it is every record in the file, and
  has to keep matching the "N QSOs" ClubLog reports.

**The windows are minute-accurate and this matters.** A date-only
implementation was written first and was wrong: T6AA's window is 19:00–20:15
on 2019-09-21, and VU2CPL worked it at 05:44–07:01 that morning — four valid
QSOs that a day-wide comparison throws away. Same for VP8STI, whose two
entries leave a valid gap his four QSOs sit inside. On his log the correct
answer is that **nothing changes**: 320 entities, 26,179 worked calls, 4,336
slots before and after, and the only genuinely invalid contacts (three V55DX
QSOs) were already uncounted because ClubLog exports them with no DXCC field.
The `local_parity` test now covers both directions — strict Swift parity with
the list cleared, and a shape assertion (entities and slots may only be lost,
never gained; QSO count fixed) with it loaded.

**Verification is a deploy away, not a test away.** The Mount Athos QSO dates
are not in the matrix, so the proof is empirical: refresh VU24DX on
`adersh@192.168.1.151` after deploying and the card should read **313**.
Every host needs a ClubLog refresh before its totals move — the fix changes
how a matrix is *built*, and stored matrices are not rebuilt on upgrade.

### DONE: My ClubLog band × mode grid — v2.13.0 on all four hosts (2026-08-30)

**Built, released and deployed.** One table replaces the two: a row per mode
class (worked and confirmed), a column per band, a `Total` column and a
`Mixed` row — RUMlog's shape. `by_band_and_mode` gained a third field,
`grid`, and 206 tests pass. v2.13.0 is published with the Windows zip, and
noderedpi4 (9 nodes), the Windows box (2), adersh (4) and vu2wj (2) all
report it; both third-party config md5s unchanged.

**Still not seen in a browser — this is the one open thread.** The counting
is covered by tests and the UI compiles, but rendering the page needs a
login, so the layout itself has never been looked at. **Open the Stats tab on
noderedpi4 and check it**, because nothing in the gate can. If you would
rather look locally, note that `config/dxca.toml`'s cluster nodes will dial
with **this station's `login_call`** and fight the Pi's session; use a config
with no `cluster_nodes` for a look-only run.

**Pinning the toolchain cost the cross targets, once.** `rust-toolchain.toml`
naming `1.96.1` makes rustup treat it as a different install from `stable`,
with its own target set — so `win-bundle.sh` stopped with *missing rust
target* even though the same compiler had built Windows binaries the day
before. Fixed with `rustup target add x86_64-pc-windows-gnu
aarch64-unknown-linux-gnu`. **Expect this again on the next pin bump**, and
add the targets straight after raising it.

Two things kept out of scope on purpose, still open: **a `Sat` column** (not
a band, needs a decision about the slot key, touches alerting) and **the band
-count award row** — `5 Band`, `6 Band`, `9 Band`, `10 Band`, `WARC`,
`Slot 26`, worked and confirmed, which is a different computation from any
grid cell.

The original note follows, for the reasoning.

**Today the Stats › My ClubLog screen shows two separate one-dimensional
tables** — *Entities per band* (mode-agnostic) and *Entities per mode*
(band-agnostic). RUMlog shows the **cross product**, and that is the ask: for
every band, the worked/confirmed split per mode.

RUMlog's shape, for reference — rows `CW wkd/cfd`, `Data wkd/cfd`,
`Phone wkd/cfd`, `Mixed wkd/cfd`; columns `Total`, then 160m…70cm, then `Sat`.

**The data is already there; this is mostly presentation.**
`DxccStatus.slots` and `.confirmed_slots` (`crates/dxca-core/src/matrix.rs`)
are `HashSet<String>` of `"20M-DATA"`-style band-mode keys, per entity. A cell
is one `count(|s| s.slots.contains("20M-CW"))` — the same closure
`by_band_and_mode_excluding` already uses for its two projections.

What has to change:

- **`by_band_and_mode` does not do what its name says.** It returns
  `BandModeStats { bands, modes }` — two independent projections, never a
  cross product. Add a third field of per-(band, mode) cells rather than
  reinterpreting either existing one; both are still needed, because they are
  exactly RUMlog's `Mixed` row and `Total` column.
- The cells iterate `bands::SELECTABLE_BANDS` × `modes::CLASSES` (`CW`,
  `PHONE`, `DATA`) — the three already match RUMlog's three, with `Mixed`
  being the mode-agnostic row we have.
- `Stats.svelte` renders the grid. Keep the "empty rows stay visible" rule
  `by_band_and_mode` already documents — a band with nothing on it is the
  most interesting row on the page, and that applies doubly per mode.
- The `exclude deleted entities` toggle must flow through, as it does now
  (`*_excluding`).

**Two things in that RUMlog screenshot are adjacent features, not this one.**
Do not let them ride along silently:

- **A `Sat` column.** Satellite is not in `SELECTABLE_BANDS` and is not a
  band — it is a propagation path. Supporting it needs a decision about where
  it lives in the slot key, and touches alerting, not just stats.
- **The band-count award row** — `5 Band`, `6 Band`, `9 Band`, `10 Band`,
  `WARC`, `Slot 26`, each worked and confirmed. These are DXCC award
  categories (entities worked on ≥N bands, WARC-only counts), a different
  computation from any cell in the grid. Worth having, separately.

### NEXT: a detailed help file — setup and use (Manoj, 2026-08-29)

**The next piece of work is a proper help document for DXCA**, covering both
setup and day-to-day use, and **especially how to get ClubLog and Telegram
working** — those two are the steps that actually block a new operator, and
neither is obvious from the outside:

- **ClubLog** needs an *app password* (not the account password), the log's
  own callsign (which may not be the login), and — separately, once per
  server and admin-only — an **API key** that fetches cty.xml and is nothing
  to do with downloading a log. That split confuses people and is exactly
  what the existing `?` tips try to explain in two sentences.
- **Telegram** needs a bot from @BotFather, its token, and a **chat id**,
  which is the part with no obvious route: there is no UI anywhere that tells
  you your own chat id. Whatever the help says here has to be a real
  procedure, not "enter your chat id".

**Ask before designing it — the format is genuinely open**, and there are two
different answers already half-present in the tree:

- A **document** — `docs/` already holds `PLAN.md`, `TELNET-INTERACTIVE.md`
  and `PHASE-ROTATION-MASK.md`, and the README is already long and
  install-focused.
- **In-app help.** v2.12.0 ported Meridian's `HelpTip` (the `?` popovers) but
  deliberately **not** its `HelpDrawer` or the backend help index behind it —
  Meridian serves a help topic per key, with a summary in the popover and a
  full body in a drawer. The `?` component here is already shaped to grow a
  "Learn more" affordance if that is the direction. See
  `~/projects/meridian/web-ui/default/src/lib/help.svelte.ts` for the
  contract, and note it is a real backend feature there, not just UI.

The two are not exclusive — a document could be the source the drawer serves.

1. **Nothing outstanding on the fleet.** All four hosts are on v2.12.1.
   `main` is one commit ahead of the tag — the embed no longer falls back to
   the login callsign when no ClubLog callsign is set. That path is
   unreachable today, so it was not worth a release; it rides the next one.

   **The VPN tunnel flaps, and both VPN hosts have now shown it.** adersh's
   v2.12.0 deploy died mid-rsync with `Connection closed`; vu2wj took three
   attempts to answer ssh at all (`Operation timed out`, then `Network is
   unreachable`, then fine). Neither is a fault in the host or the script —
   the swap happens only after the transfer completes, so a drop leaves the
   host on its old binary and still serving. **Verify, then simply retry.**

   **The deploy recipe that worked, for next time.** Both VPN hosts:
   `deploy/pi-deploy.sh --no-seed <user>@<ip>`, with `data/dxca.db` copied to
   `dxca.db.pre-<version>` FIRST. `--no-seed` is not optional — their databases
   carry their own ClubLog credentials and Telegram tokens. Verify after with
   the version, the node count, the alert row count and an **md5 of their
   `config/dxca.toml` taken before and after**, which is what actually proves
   `--no-seed` did its job.

   **Expect the transfer to fail at least once over the VPN.** The adersh
   deploy died with `Connection closed by 192.168.1.151 port 22` part-way
   through rsync. Nothing was harmed — the swap happens after the transfer, so
   the host was still on its old binary and serving — but **check before
   retrying** rather than assuming: `systemctl is-active dxca`, the binary's
   size and date, and the version the API reports. A plain retry then worked.
2. **DONE — the network-failure fix is merged** (`afc9fd0`). `lib/api.ts` no
   longer lets `fetch` throw; an unreachable server arrives as `status: 0`,
   which every one of the forty call sites already treats as failure, and the
   `ConfigGate` component gives the Server pages a real error state instead of
   rendering nothing. Not deployed yet — it is on `main`, past v2.12.0.

   **Two things worth carrying forward from it.**

   *A Vite dev proxy hides this bug class.* With the proxy in front, an
   upstream that is down returns **HTTP 500** and `fetch` resolves — so the
   dev setup exercises the non-200 path and never the throw. Against a
   genuinely unconnectable origin (`http://127.0.0.1:1/`), raw `fetch` gives
   `TypeError: Failed to fetch`. Any future test of "server unreachable"
   has to bypass the proxy or it is testing the wrong thing.

   *Changing a shared primitive's failure contract needs a caller sweep.*
   Making `api()` return instead of throw silently changed what an UNGUARDED
   `x = r.json` does. Two of forty sites were unguarded; one fed the shared
   status store and would have made the header pill claim "0/0 nodes".

**Two things a reviewer should look at first**, because they are the parts that
touch existing data:

- The `snr_db` migration has **already run on noderedpi4**. It is additive and
  verified (199 rows preserved, all NULL), and there is a
  `dxca.db.post-snr-migration` snapshot beside the live file — but note it is a
  POST-migration copy. I ran the migration without taking a pre-migration
  backup first, which the `.pre-v2.4.0` / `.pre-v2.9.0` files show is the habit
  here. **Take the backup first on the next one.**
- The `notify_spotter_kind` adoption. Read `notify_config` and
  `set_notify_config` together: the empty-string default is what makes adopting
  the old boolean possible, and writing both fields in step is what stops the
  adoption re-firing over a deliberate choice. Both pinned by tests.

**Hard-refresh before judging any UI change.** Browser cache cost an earlier
session a wrong diagnosis; see the stale-asset note further down.


**SHIPPED as v2.9.0, then v2.9.1 (2026-08-29): the Stats tab** (coloured in
2.9.1), the Windows config import (with sibling detection in 2.9.1), and the
mask groundwork. On all three Pis,
[released](https://github.com/vu2cpl/dxca/releases/tag/v2.9.0) with the
Windows zip. The `station_json` migration ran cleanly on every box —
alert histories preserved (189 on noderedpi4, 215 on adersh), accounts and
matrices intact, `dxca.db.pre-v2.9.0` left on each as a rollback.

Total spots held, plus breakdowns by band, mode and source.
`GET /api/spot-stats` aggregates across the **whole ring**, not the 500 the
Spots screen holds — on a busy feed that is five minutes, which would answer
a far smaller question than the one asked, and would change on every reload.
Band comes from the frequency and mode from the spot, so the aggregation is
user-independent and needs no session.

**Bars, not pie charts, on purpose.** The job is magnitude comparison across
up to fifteen categories with long names; a fifteen-slice pie cannot be
compared by eye or hold its labels. Each chart is a single series, so there
is no legend — the heading names it, and colour carries no meaning beyond
"this is the bar". Fifteen hues for fifteen bands would encode identity the
labels already carry and fail on colour-vision grounds for nothing.

One layout detail worth keeping: the grid lives on the **chart**, with rows
as `display: contents`, so the label column sizes to the longest name in
that chart. A fixed width truncated `UberSDR CWskim` to `UberSDR C…` — in a
chart about which node carried what, losing the node's name is exactly the
wrong thing to lose. Caught by looking at the render, not by a test.

**Colour: one hue per chart, and every hue validated rather than chosen.**
Teal for bands, violet for modes, pink for sources — those three because
everything else is already spoken for in this app: red, orange and yellow
are the DXCC/Slot/Mode alert levels, blue is New Band *and* the accent, and
green is the `ok` status. An orange bar in Stats would read as "New Slot".
Fifteen hues for fifteen bands was never on the table: it encodes identity
the labels already carry and fails on colour-vision grounds for nothing.

The steps came from the palette validator, not from taste. The first teal
(`#1b7c83`) **failed the chroma floor** — it reads gray — and dark mode
needed its **own** steps, because flipping the light ones puts violet
outside the lightness band. Light `#0891b2 / #6639ba / #bf3989`, dark
`#22a7b3 / #a371f7 / #db61a2`; both sets pass all six checks.


**Not a bug, but it looked exactly like one (2026-08-29): a UI-only change
appeared not to reach the binary.** After editing only a `.svelte` file and
running `just web` + `cargo build`, the browser rendered the OLD layout;
touching a `.rs` file "fixed" it. That reads as `include_dir!` failing to
invalidate — which would mean every deploy could ship a **stale dashboard**,
silently.

It was **browser cache**. No hard reload had happened between the edit and
the screenshot. Tested properly afterwards: marker into a `.svelte`, `just
web`, `cargo build` with **no** `.rs` touched, and the marker appeared in the
bundle the server handed out. `build.rs`'s `cargo:rerun-if-changed` on
`web-ui/dist` works, because vite rewrites the directory and moves its mtime.

Worth keeping because the failure mode is so plausible and the consequence so
bad. **When a UI change seems not to have deployed, hard-reload before
suspecting the build** — on Safari, Shift-click reload. The cheap check is
comparing the `index-*.js` name the server serves against the one in
`web-ui/dist/assets/`; the deploy scripts now get that check by habit.

**The band mask now runs on sun PHASES around a tunable grey-line window,
and milestone 4 is built (2026-08-29).** Manoj: *"m4 next and add a twilight
setting with default 45 minutes. this is user variable to get greyline timing
which can vary. see meridian implementation."*

**The elevation model was wrong about the thing that matters most.** It got
day and night right and could not express the **grey line** at all — the
narrow window either side of the terminator where the D layer has collapsed
but the F layer is still lit, which is when the DX is actually worked on 160m
and 80m. A fixed number of degrees is a wildly different amount of *time*
depending on latitude: measured at the June solstice, 45 minutes before
sunset is about 9° up in Bengaluru and about 5° in Munich. One threshold is
wrong at one end or the other, always.

So the model now resolves **Dawn / Day / Dusk / Night against the real
sunrise and sunset** for that place and day, with the window in **minutes**
and set by the operator. `bands::plausible_at(band, elevation)` became
`bands::plausible_in(band, phase)`; the elevation table became a phase table.
`solar::elevation` stays — it is still the honest answer to "how high is the
sun" and its tests still pin the refraction bias — but nothing in the mask
calls it any more.

**Ported from Meridian's `meridian-core::geo`**, defaults included, so the two
programs cannot disagree about what phase it is. The pieces: `SunPhase`,
`sun_times`, `phase`, the "Almanac for Computers" horizon solve, and Howard
Hinnant's civil-calendar helpers. Note the two algorithms now living side by
side in `solar.rs` and **why that is deliberate** — `elevation` is NOAA and
geometric; `event_ut_hours` solves directly for the horizon crossing at a
90.833° zenith that includes refraction. Solving one from the other would
need iteration and would behave badly on exactly the polar days where the
elevation curve never crosses zero.

**The tuning knob moved out of the source and into the operator's hands**,
which is what made it safe to skip the "watch it for a week" step this plan
had insisted on before milestone 4. A model that disagrees with the bands is
now something Manoj adjusts on My ClubLog, not something he waits for a
release to fix.

**Milestone 4, both halves:**

- **Hide mode**, a `<select>` beside the tickbox, shown only once the mask is
  on, with **dim as the default** — a corrupted or half-written localStorage
  preference lands on dim, never on hide. The `N hidden` count is derived
  from the rows *before* hiding, so the number survives the thing it counts.
  A mask that removed rows and lost count of them would be precisely the
  silent-filter failure this feature exists to avoid.
- **Telegram** (`notify_respect_band_mask`), off by default, narrowed
  separately from the screen. **New DXCC is exempt** — the screen never dims
  it and Telegram never holds it, and the reason is stronger here: a dimmed
  row is one hover from being read, a held alert is a spot never learned
  about. And **no opinion never suppresses**: no locator, or an unmodelled
  band, sends as before. Four cases pinned in
  `telegram_band_mask_fails_open`.

**Bounds are refused, not clamped.** 5–180 minutes; outside that the PUT
returns 400 with the range in the message. Silently changing a number the
operator typed is how they stop trusting the screen.

**Watch the `StationConfig` default.** `#[serde(default)]` on the struct
would have made a missing `greyline_window_min` **zero**, which abolishes the
grey line rather than defaulting it — every existing account has a row
without that key. Hence the hand-written `Default` and the explicit
`#[serde(default = "default_greyline_window_min")]`, and a test asserting a
round trip still reads 45.

**Verified in a browser against the real pipeline**, same isolated local
instance as milestone 3: the phase badge read Day, the mode selector took
10 rows to 5 with the badge changing "5 dimmed" → "5 hidden", the stepper
moved 45 → 55, the save round-tripped, and 2 and 600 minutes were both
refused with the range in the message.

**Not exercised on real data:** the Telegram narrowing has never held an
actual alert, and the New DXCC exemption still needs a loaded log to reach.

**Phase-rotation band mask: milestone 3 built and verified in a browser
(2026-08-29).** `docs/PHASE-ROTATION-MASK.md` has the full record. A **Band
mask** tickbox on Spots, off by default, shown only once a locator is set;
masked rows dim to 45% and come back to full on hover; an `N dimmed` badge
sits beside the spot count. The mask takes no part in the `visible` filter,
so it cannot empty the table — the lesson from the alert-level filter.

**Verified against the real pipeline, not by reading.** A local instance in
the session scratchpad with **no cluster_nodes** — logging in as VU2CPL
would fight the shack Pis for the same node session — fed synthetic WSJT-X
UDP packets by **`scripts/feed-spots.py`**, written off `dxca-core/src/wsjtx.rs`
(magic, schema, type, then u32-length UTF-8 strings; a Status sets the dial
frequency the Decodes are relative to). At 11:26 IST from MK82 the 160M and
40M rows dimmed, 15M and 10M did not, the badge read `5 dimmed`, hover
restored a row, both themes were legible and the preference survived a
reload. **First attempt started the server from the repo root and it picked
up `config/dxca.toml` — the burn-in config — connecting to all five real
nodes as VU2CPL for about twenty seconds before it was killed.** The config
path is hard-coded relative to the working directory (`config::DEFAULT_PATH`),
so an isolated run needs its own directory, not a flag. Worth knowing before
the next local test.

**That verification found a coupling bug worth more than the milestone.**
`band_open` was annotated inside the classification branch, and
`users::classify` returns `None` for an account with no ClubLog matrix — so
the mask's precondition had silently become *"has a ClubLog log"* rather
than *"has a locator"*, and the Band column was empty for those accounts
too. A band is a property of the spot's frequency, not of anyone's log.
`annotate_spot` now derives it unconditionally and classification adds only
the alert level, DXCC name and beacon flag on top. **Still unexercised:** the
New DXCC exemption needs a loaded log to produce a flagged spot, so the
local check could not reach it.

**Stats: the three charts now share one fixed label column (2026-08-29).**
Manoj sent a screenshot of the live page and said the labels need to be
fixed width. He was right and the original comment in `Stats.svelte` argued
the opposite: `max-content` sized each chart to its **own** longest name, so
`160M`, `FT8` and `DB0SUE` each started their bars at a different x. That
reads fine one chart at a time and ragged down the page — and the charts are
stacked, so they get compared whether or not they were meant to be. Now
`7.5rem` for all three, with labels **wrapping** rather than truncating:
verified that a 25-character name goes to two lines, is not clipped, and
does not move the bars. Truncating a node's name in a chart about which node
carried what would be exactly the wrong thing to lose.

The same screenshot showed **"in memory— about 32 min"** with no space.
Svelte trims the leading whitespace of an `{#if}` block's content wherever
it is written — moving it onto the tag's own line does not help — so it
needs an explicit `{' '}`. The space cannot live outside the block either:
that leaves "memory ." on an instance with no span yet. `span()` also
returned "0 min" on a fresh instance, so it now returns the whole phrase and
says "under a minute".

**Windows install location fixed, and the first real Windows test found a
second bug (2026-08-29).** Manoj ran the new installer on his own machine.
The fixed location worked; the **one-time import did not** — it offered no
previous install and he copied `config\` and `data\` into `C:\DXCA` by hand.

The cause was a batch parsing bug in the scheduled-task lookup that had
**never once worked**, in either version of this script:

```
for /f "tokens=2*" %%a in (... findstr /c:"Task To Run:") do set "OLDEXE=%%b"
```

The line reads `Task To Run:   C:\somewhere\run-dxca.cmd`. Token 2 is `To`,
so the `*` remainder begins at `Run:` — `OLDEXE` came out as
`Run:            C:\somewhere\run-dxca.cmd`, which `%%~dpp` cannot make a
folder out of. Splitting on `:` is not the fix either; the path has one.
The working idiom is `!VAR:*Task To Run:=!`, which deletes everything up to
and including the needle and leaves the path whatever it contains, followed
by a `for /f "tokens=* delims= "` pass to strip schtasks' padding.

**Why nothing reported it:** detection failing is *designed* to fall through
to a prompt rather than complain, so a permanently broken lookup and a
machine with no previous install look identical from the outside. The
sibling-folder scan — the fallback that was meant to cover this — only fires
when the new zip is unpacked beside the old install, and it was not.

Two lessons worth keeping. A "convenience" path that silently degrades needs
a way to tell *broken* from *nothing to find*; and `tokens=N*` is worth
spelling out on paper before trusting it, because it splits where the
delimiters are, not where the label ends.

Manoj's own machine is already migrated by hand, so `C:\DXCA` holds his
config and database and every future run is detected as an upgrade. Worth
confirming once that the task points at the new location:
`schtasks /query /tn dxca /v /fo list | findstr "Task To Run"` should read
`C:\DXCA\run-dxca.cmd`. The fix matters for the next machine, not his.

**NEEDS TESTING ON WINDOWS (2026-08-29): DXCA now installs to a fixed
location, `C:\DXCA`.** Written but **not run** — there is no Windows machine
in this workflow, so the batch is unverified beyond a paren-balance check.
**Test before trusting it.**

**The bug this fixes is data loss, not inconvenience.** `INSTALLDIR=%~dp0`
meant DXCA ran from whatever folder the installer sat in, and every release
unzips into its own version-named folder (`dxca-2.8.0-windows-x64\`). So
installing a new version was always a *fresh* install: new empty database,
and the account, ClubLog credentials, log matrix and alert history left
orphaned in the previous version's folder. That is the "every install needs
reconfiguring" report.

The first attempt at this (v2.9.0/v2.9.1) was an **import prompt** — detect
the old folder, offer to copy its config and database forward. It worked,
but it treated the symptom: every single upgrade still had a question to
answer and a folder to find, forever. Manoj's "first fix the windows
installer" is what turned it into the real fix.

**The script now separates two things it had conflated:**

| | |
|---|---|
| `SRCDIR=%~dp0` | where the zip was unpacked — holds the new `dxca.exe`, disposable |
| `INSTALLDIR=%SystemDrive%\DXCA` | where DXCA lives and runs — permanent |

An upgrade is now: stop the task, copy `SRCDIR\dxca.exe` over
`INSTALLDIR\dxca.exe`, re-register. `config\` and `data\` are already there
and are not touched. **Nothing is asked.** `%SystemDrive%` rather than a
literal `C:` for the machine that boots from another letter.

`C:\DXCA` over `%ProgramData%\DXCA` was Manoj's call, and it is the right
one for this audience: ProgramData is hidden in Explorer by default, so
every "send me your run.log" exchange would start with unhiding system
folders. The security argument for ProgramData is real, though — a folder
created at the root of `C:` inherits the drive root's ACL, which lets any
standard user write inside it, and `dxca.exe` is launched by a LOCAL SYSTEM
task. A user-writable SYSTEM-run binary is a privilege-escalation path. So
the installer runs `icacls /inheritance:r` on the folder it creates, granting
Administrators and SYSTEM full control and Users read-only, by **well-known
SID** (`*S-1-5-32-544`, `*S-1-5-18`, `*S-1-5-32-545`) because group names are
translated on a localised Windows. It warns rather than fails if that does
not take.

**Two ordering constraints, both load-bearing:**

- The `mkdir` + `icacls` block sits **before** the import block, because the
  import does `mkdir %INSTALLDIR%\config` and cmd creates intermediate
  folders on the way. Left where it was written, the lockdown would have
  been skipped on exactly the installs that perform an import.
- The `copy` of the new exe happens **after** the task is stopped and the
  port is confirmed free. Copying over a running exe fails, and failing
  after the service is down would leave the machine with no DXCA at all.

**The import block survives, but now runs once.** It fires only when
`C:\DXCA` holds no install, which after the first migration is never again.
It looks in three places, in order: the scheduled task's path, the unzipped
folder itself (someone who unzipped over their old install), and any
`dxca-*-windows-x64` sibling of the unzipped folder, newest first. The
sibling check is the reliable one — `schtasks /v /fo list` is English-only
and its encoding varies by Windows build — and detection failing falls back
to a prompt rather than blocking. The database is copied first; a half-done
import that took the config but not the accounts would be the worst outcome.
The import sets `UPGRADE=1` so the config is then treated as the operator's
and never rewritten. The old folder is left untouched and doubles as a
backup.

**Harmless case worth knowing:** an operator who followed the old advice and
kept versions under `C:\dxca` finds that folder *is* the install location —
Windows ignores the capitals. Their old version folders end up sitting
inside `C:\DXCA`. Untidy, not broken. Note that the pre-existing folder
skips the `icacls` step, since re-permissioning a folder the operator made
themselves is not the installer's business.

Docs updated in the same cycle: `deploy/windows/README-WINDOWS.txt`
(sections 3, 4 and 4a rewritten), the top-level `README.md` Windows and
Updating sections, and `uninstall-dxca.cmd`, which now reports `C:\DXCA`
rather than its own folder.


**Milestones 1–2 BUILT (2026-08-29), 3–4 designed only: the phase-rotation
spot mask** —
[`docs/PHASE-ROTATION-MASK.md`](docs/PHASE-ROTATION-MASK.md). Manoj's
request, taking Meridian's locator-driven band rotation and applying it to
the spot feed: stop showing New-DXCC 160m spots at local midday.

**M1** shipped the pure maths: `grid::parse` (Maidenhead → the centre of the
square) and `solar::elevation` (NOAA solar position). **M2** added
`bands::plausible_at`, which **fails open** — unknown bands and the ones the
model says nothing about (30M, 6M up) are never masked — plus a
`station_json` per-user blob for the locator (added via the migration
mechanism), a validating `PUT /api/config/me/station`, and a `band_open`
annotation on spots.

**Nothing is filtered, dimmed or hidden.** The server offers advice; no
client acts on it. `band_open` appears only for an account with a valid
locator, and an unparseable one behaves as none — asserted, because Manoj's
standing requirement is that this stays an opt-in tickbox, default off,
nothing imposed. Remaining: the UI (M3) and the Telegram narrowing (M4).

Sunrise agrees with published times to within 5–8 minutes, consistently
late, which is atmospheric refraction — almanacs quote apparent sunrise,
this returns the true geometric one. Left uncorrected on purpose and pinned
by a test that explains itself. The Tromsø tests (midnight sun, polar night)
are the argument for elevation over clock time made executable.

**The load-bearing decisions, if you read nothing else:** use **sun
elevation**, not clock time — a fixed local-time rule is wrong by up to six
hours across a European year and breaks entirely above the Arctic circle.
Default to **dim, not hide**, because the cost of concealing a workable rare
one vastly exceeds the cost of showing an unworkable one. **Never mask New
DXCC** by default. And always show a **count of what was masked** — today's
finding about the alert-level filter is the direct precedent: a filter that
silently empties the screen reads as broken.

Stopping after milestone 3 (dim mode on the Spots screen) is a good outcome;
the threshold tuning it produces matters more than the remaining code.


**SHIPPED as v2.8.0 (2026-08-29): WSJT-X mode names, and telnet ECHO
negotiation.** On all three Pis,
[released](https://github.com/vu2cpl/dxca/releases/tag/v2.8.0) with the
Windows zip. Verified on the live shack server that a client which never
logs in receives **zero `0xFF` bytes** — no negotiation reaches a logger.

**The Windows/WSJT-X "no mode" bug is fixed, and it was real.** WSJT-X
reports a decode's mode as the single character it prints — `~` for FT8 —
not as a name, and DXCA passed that straight through. Confirmed in the
committed capture: `crates/dxca-core/tests/vectors/wsjtx/type02-1.bin` ends
`\x00\x00\x00\x01~`. MSHV sends `"FT8"` as a name, which is why the Pi
looked fine and only the Windows/WSJT-X install showed the problem.
`modes::from_decoder_char` maps the markers; anything unmapped falls back to
the **Status** message's mode, which is a proper name from the decoder
itself, and only then to band-plan inference.
`a_real_wsjtx_decode_reports_ft8_not_a_tilde` drives the genuine capture
through the pipeline and was verified by breaking it.

**Telnet ECHO negotiation** closes the last item on the interactive-telnet
feature: the password is no longer echoed, and the feed no longer shreds the
operator's typing. **Offered only after `LOGIN` is typed**, never on connect,
so a logger still never sees a negotiation byte — asserted on both the
banner and the feed. Server echo engages only on an explicit `DO ECHO`.

**A bug that only character mode could expose:** RFC 854 sends a bare CR as
`CR NUL`, and the NUL was surviving the line split to prefix the next
command — `sh/nodes` arrived as `\0sh/nodes` and was refused. In line mode
the stray byte never mattered. Found by driving a real `telnet` through a
pty; no unit test would have generated `CR NUL`. **The lesson repeats:
every bug in this feature has been found by using it, not by testing it.**


**SHIPPED as v2.7.2 (2026-08-29): BYE now disconnects.** **Live on all three Pis**,
[released](https://github.com/vu2cpl/dxca/releases/tag/v2.7.2) with the
Windows zip. VU2WJ's box was briefly unreachable during the first pass —
no ping, no ssh, while the VPN was up and adersh answered normally, so the
box was off rather than the tunnel being down — and took the deploy a few
minutes later when it came back.

Reported from the field: *"bye ] and quit didnt quit"*. Two things. `Ctrl-]`
is a control character and a literal `]` is easily typed instead, which just
sends text to the server — but the operator should never have needed the
telnet escape. `BYE` used to **log out while keeping the socket open**, on
the reasoning that the spot feed is what most clients want. For a human that
is wrong and surprising: every real cluster disconnects on `BYE`, and the
old behaviour left an operator watching a streaming feed with no obvious
exit. It now says `73 <call>.` and hangs up.

**The logger protection is unchanged and still tested**: an anonymous
session's `BYE` is ignored entirely, so a logger that happens to transmit it
is never hung up on. That distinction is the whole reason `BYE` was gated on
authentication in the first place.


**SHIPPED as v2.7.1 (2026-08-29): the telnet feed is held during a command
reply.** Live on noderedpi4 and
[released](https://github.com/vu2cpl/dxca/releases/tag/v2.7.1) with the
Windows zip. **adersh and vu2wj remain on v2.7.0** — no rush, the fix only
affects interactive telnet, which is off on both.

The first real `SH/DX` against DB0SUE **worked end to end** and leaked no
history (see `docs/TELNET-INTERACTIVE.md` for the evidence — the naive check
looks like a leak and isn't). What it exposed was readability: live spots
landed between the rows of the `SH/DX` table. The feed is now held and
**buffered** from submit until the reply goes quiet (2.5 s grace, 20 s hard
cap, 500-line buffer), then flushed — delayed, never dropped. Anonymous
sessions are never held, which has its own test.

**Still open, and it needs `IAC WILL ECHO`:** the operator's *typing* is
shredded by the feed too. In line mode the client echoes locally and sends
nothing until Enter, so the server cannot know a line is in progress. The
same negotiation would also stop the password being echoed. **One piece of
work fixes both** — that is the next thing worth doing on this feature.


**SHIPPED as v2.7.0 (2026-08-28): Telegram manual-only, and the DXCC
toggle inverted.** Live on noderedpi4 (9 nodes),
[released](https://github.com/vu2cpl/dxca/releases/tag/v2.7.0) with the
Windows zip. **adersh followed the same evening** (`--no-seed`, v2.5.0 → v2.7.0 in one
step): 114 alert rows, account, cty and LoTW intact, four nodes Live.
On both boxes the stored notify row has **no `notify_manual_only` key at
all**, which is exactly right — it deserializes to off, so Telegram behaves
as it did until someone ticks the box.

*Field note: **95%** of adersh's feed is skimmer spots (21 of 22 sampled),
against ~74% on noderedpi4. Manual-only will be a much bigger change on his
station than on the shack's.*

- **Telegram manual-only.** `NotifyUserConfig::notify_manual_only`, applied
  in `fan_out` through `passes_skimmer()` — the same predicate idiom as
  `passes_band_mode`, so it is unit-testable rather than inline. Lives in
  the notify JSON blob, so **no migration**: an old row deserializes to
  `false` and behaves exactly as before, which has its own test. The point
  of keeping it independent of the Spots screen's Manual-only is the same
  as for band/mode narrowing — watch everything on screen, be interrupted
  only by people.
- **The DXCC toggle is inverted**, at Manoj's request: current-only is now
  the **default** (it is what the ARRL publishes and what an operator
  compares against) and the tickbox reads *include deleted entities*. The
  shared preference key changed from `currentOnly` to `includeDeleted`; a
  stale stored value simply reads as `false`, which is the wanted default.
- **Placement**: on the Spots station card the tickbox moved to the far
  right of the row, after the numbers — wedged between the callsign and the
  first total it broke the label/number/caption rhythm and read as a stray
  control. **My ClubLog's placement was left alone** — Manoj said it was
  fine there, so only its label and default changed.

Earlier the same evening, shipped as **v2.6.0**: skimmer identification and
the Spots "Manual only" filter.

Asked for as "how do I skip skimmers?" The answer through the telnet
passthrough is *you can't, deliberately*: `accept/rbn` / `reject/rbn` are
node-side filters, and DXCA shares one session per node with every account
and with the spot pipeline, so setting one would narrow everyone's feed and
persist on the node account. That refusal is correct — but it left the need
unmet, and the data was already being thrown away.

`ParsedSpot::spotter_is_skimmer` existed and was used only to decide
`is_cq`, then discarded — the same bug class as the spotter itself.
`Spot::is_skimmer` now carries it. **The marker matters because the parser
strips the `-#` to keep callsigns readable**, so without the flag `W3LPL`
(the operator) and `W3LPL-#` (his skimmer) are identical on screen. The
Spots table shows a `#` after the spotter and a **Manual only** tickbox
hides them.

Verified in a browser against a fake node emitting the *same callsign both
ways*: 6 spots → 3 with the box ticked, W3LPL's hand-typed spot surviving
while W3LPL's skimmer spot was filtered.

**Not done, and the obvious next step:** the same narrowing for Telegram.
`NotifyUserConfig` already has band/mode narrowing; a `notify_manual_only`
flag would slot in beside it, so alerts can be human-spots-only without
touching the Spots screen.


**SHIPPED as v2.5.0 (2026-08-28): "current entities only" for award
totals, and a Telegram format change.** **Live on both Pis** — noderedpi4 (9 nodes) and
`adersh@192.168.1.151` (4 nodes, account and reference data intact).
[Released](https://github.com/vu2cpl/dxca/releases/tag/v2.5.0) with the
Windows zip, per the new standing rule above. No schema change since
v2.4.0, so upgrading is a binary swap.

**Current-entities toggle.** DXCC has 62 deleted entities in cty.xml (Abu
Ail, Aldabra, Blenheim Reef, British North Borneo…). cty.xml has always
carried `<deleted>`, and the parser has always read it — but only to decide
whether to build a prefix rule, after which it was discarded. `DxccEntity`
now keeps the flag, `DxccResolver::deleted_adifs()` exposes the set, and
`LogMatrix` gained `stats_excluding` / `by_band_and_mode_excluding`. The
matrix itself stays resolver-free — it stores what was *worked*, not what
currently *scores* — so the caller, which holds both, supplies the set.

`/api/me/station` sends **both** sets (`stats` + `stats_current`,
`by_band_mode` + `by_band_mode_current`), so the tickbox is instant; the
payload is a dozen integers and a round trip per toggle would cost more.
The preference is shared between the Spots station card and the My ClubLog
statistics via `web-ui/src/lib/awards.svelte.ts`, deliberately — two cards
disagreeing about which entities count is worse than either answer alone.
`*_current` is **null when no cty.xml is loaded**, and the tickbox hides
itself: showing unfiltered numbers under a "current only" label would be a
quiet lie.

Verified end to end against a seeded matrix (3 current + 4 deleted
entities) with the real 402-entity cty.xml: DXCC 7→3, Challenge 42→30,
confirmed 26→18, and the per-band table dropping 40M/20M/15M from 7 to 3
while 30M correctly stayed at 3 (the deleted entities were never worked
there).

**Telegram format**, at Manoj's request: `Spotted by: X via Y` became
`Spotter: X   Node: Y` on its own line, with the time below it. Labelled
rather than prose because those two labels are what you scan for on a
phone. A local decode shows only `Node:` — an empty `Spotter:` would read
as missing data rather than as "us".


*(Resolved 2026-08-28: the two world-readable database copies left in
`adersh@192.168.1.151:/tmp` during the v2.4.0 migration check were deleted
as soon as the VPN came back; his box now has nothing of mine in `/tmp`,
and the intended `dxca.db.pre-v2.4.0` backup remains at 0600. The lesson
stands and is worth keeping: **copying a database off a box and deleting
the copy belong in one command**, not two steps separated by a network that
can vanish — a `chmod 644` on a file holding ClubLog passwords and Telegram
tokens should never outlive the scp that needed it.)*


**DEPLOYED as v2.4.0 (2026-08-28): spotter attribution + spots search.**
Live on noderedpi4 and verified against the real feed, not only tests: 63 of
73 spots carried a spotter, the migration preserved all 91 alert rows, and
`dxca.db.pre-v2.4.0` sits beside the database on the Pi as a rollback.
Three of four requests from Manoj; the fourth ("local spots not showing
modes") is **unresolved and still his to reproduce** — the live API shows
MSHV spots carrying `mode:"FT8", mode_inferred:false`, so the symptom did
not match the data and he said he would check where he was seeing it.

- **`Spot::spotter`** is a new `Option<String>` on the core model. The
  parser always extracted the spotting station; `synthetic_spot` dropped it,
  so every relayed spot was attributed to the *node* that carried it. A
  HamAlert or N2WQ feed says nothing about whose receiver heard the DX,
  which was the whole complaint. `None` for locally decoded spots.
- **Telegram** now ends `Spotted by: VU2XYZ via N2WQ-2  at 1428Z`. The
  "via" clause is suppressed when spotter and node are the same, so a
  W3LPL-fed W3LPL spot does not read "W3LPL via W3LPL". Time is the spot's
  own `hhmm()` in UTC, not delivery time.
- **Spots table** gained a sortable Spotter column beside Source, and a
  search box matching either the DX call or the spotter.

Verified in a browser against a fake DXSpider node emitting varied spotters,
in both themes — not only in tests, per the invisible-prompt lesson.

**The history carries it too** (asked for straight after): `alerts_sent`
gained a `spotter` column, and with it **`db.rs` finally has a migration
step**. `CREATE TABLE IF NOT EXISTS` is a no-op on a database that already
exists, so a new column in `SCHEMA` reaches fresh installs only — every
install in the field would have kept the old shape and then failed at the
first query naming the column. `migrate()` walks `ADDED_COLUMNS`, checks
`PRAGMA table_info`, and issues `ALTER TABLE ... ADD COLUMN` for whatever is
missing. Additive only, on purpose: `ADD COLUMN` is the one change SQLite
makes without rewriting the table, and a defaulted column cannot invalidate
an existing row. Anything needing a drop, rename or retype wants a real
versioned migration instead — do not stretch this.

`opening_an_old_database_adds_the_spotter_column_without_losing_rows` builds
a database with the **pre-migration** shape by hand, opens it through
`Db::open`, and checks the column appears, the old row survives, a new row
round-trips, and a second open is a no-op. **It earned its keep immediately**
— it caught a parameter-order bug where the spotter string was being written
into the `delivered` column, which no compiler would have flagged and which
would have corrupted every alert row in production.


**KNOWN, ACCEPTED (2026-08-28): the Spots level filter is usually empty —
and it is NOT a 2.3.x regression.** Reported as "some issue in 2.3.1", so
worth recording plainly: nothing in the v2.3.0/v2.3.1 telnet work touches
this. The filter and the 500-spot backfill both date from 2026-08-27
(`4886f7e`, `02267a1`).

The behaviour is deliberate — picking "New DXCC" makes the feed a New-DXCC
feed, not "everything, DXCC highlighted", and the comment in
`Dashboard.svelte` says so. Selecting every pill still hides *unflagged*
spots, which is why it does not help. The problem is arithmetic:

- the Spots screen backfills **500 spots** (`/api/spots?limit=500`);
- with nine nodes live the feed runs at **~105 spots/min**, so 500 spots is
  **~4.8 minutes** of history;
- genuinely new spots are rare — 24 Telegram alerts in six hours, ~4/hour;
- expected flagged spots inside a 4.8-minute window ≈ **0.3**, so roughly
  three times in four the honest answer is zero.

A five-minute keyhole onto something that happens every fifteen minutes.

**The backend is not implicated** and this was checked before blaming the
UI: the matrix holds 56,836 QSOs (refreshed 2026-08-28 11:41) and
classification demonstrably works — T5FE newBand 160M at 19:50, RI1FJL
newMode at 19:47, 24 alerts in six hours.

**Manoj's call, 2026-08-28: leave it.** Working as designed, not worth
changing now. If it is ever revisited, the fix is **server-side filtering**:
the ring holds 5000 spots (~48 minutes at this rate) and the server already
classifies per user, so a `level=` parameter on `/api/spots` applied after
`annotate_spot` gives ten times the window with a *smaller* payload than
raising the backfill limit. The weaker alternatives are a bigger limit, or
having the UI retain flagged spots as they age out.


**Milestones 1–3 BUILT (2026-08-28), 4 (spotting) deliberately not:
interactive telnet with cluster-command passthrough** —
[`docs/TELNET-INTERACTIVE.md`](docs/TELNET-INTERACTIVE.md). **Live on
noderedpi4 since 2026-08-28** as v2.3.1 with `telnet_interactive = true`;
**still off on `adersh@192.168.1.151`**, which remains on 2.2.2.

**Verified against the production server, not just fakes:** an anonymous
session throwing a bare callsign, `set/name`, `sh/dx` and `BYE` at port 7575
got **zero** non-spot bytes back while three real spots flowed through it —
so RUMlog is genuinely unaffected — and RUMlog itself reconnected on its own
after the restart (`telnet_clients: 1`). `LOGIN VU2CPL` prompts for a
password, and a wrong one is refused without revealing which half was wrong.

**v2.3.1 fixed the first field bug: "it didn't ask for password".** The
protocol was fine — a real telnet client driven through a pty got the
prompt — but nothing in the banner said `LOGIN` existed, and the
newline-less `Password: ` prompt got a spot glued to it and scrolled away.
Banner now advertises the verb; the spot feed pauses for that one session
while a password is outstanding. **The lesson: every test read the socket,
where the prompt was plainly there. Nothing tested what a human watching a
scrolling terminal sees.**

**What is still unproven: a logged-in `SH/DX` against a real node.** That
needs Manoj's account password, so it was left for him. Worth doing with the
Spots screen open beside it — the one thing the fake nodes cannot show is
that a real DB0SUE history burst stays out of the live feed.

**M3** is the passthrough itself. `commands.rs` canonicalizes an abbreviated
verb against a table of ~120 DXSpider commands and allows only the read-only
tier; `telnetcmd.rs` holds per-session state (current node, reply channel)
and joins policy → router → nodes; `NodeEventFilter` gives the router first
refusal on every node event. Three things worth knowing before touching it:
**the node is sent the canonical form** (`sh/dx 5` goes out as `SHOW/DX 5`),
because judging one string and running another is a hole, not a nicety;
**interception happens before the status counters**, so a history query does
not inflate a node's spot count; and **the design's original rule about
spots was wrong** — the doc said `ClientEvent::Spot` should always reach the
pipeline, but a `SHOW/DX` reply *is* spots, hours old, so while a window is
open every event from that node belongs to the requester. That reversal is
the single most important thing in the feature, and
`sh_dx_history_reaches_the_asker_and_nothing_else` was verified by
deliberately breaking it (remove `set_event_filter` and it fails with the
leaked callsigns listed).

**M2** added the login gate: `LOGIN <callsign>` → `Password:` → argon2
against the accounts table (via `spawn_blocking`; verifying on the async
runtime would stall every other session's spots). Gated by the new
`telnet_interactive` config key, **default false** — the port is
unauthenticated and node sessions carry the shack callsign, so it never
arrives switched on. **Login is an opt-in verb, not a prompt on connect**,
which is a deliberate change from the design: the loggers on 7575 were set
up against a server that never prompted, and a 45 s capture on the Pi showed
an established RUMlog session sends nothing at all, but connect-time
behaviour can't be seen without disconnecting a live logger. An opt-in verb
makes that unknowable question irrelevant. `an_anonymous_session_is_answered_with_silence_and_spots`
is the regression guard for every existing logger and should never be
deleted. **An authenticated session still cannot do anything** — commands
are M3.

M1 shipped the
router (`cmdrouter.rs`: per-node queue, response window, quiet + hard
timers — a pure state machine taking `now_ms` and returning actions, so it
tests without sockets or a clock) plus `NodeManager::send_line()` and
`subscribe_lines()`, and the event loop now publishes node lines instead of
discarding them. 13 new tests, 119 passing workspace-wide. **Nothing is
user-facing** — no telnet session, no auth, nothing subscribes in
production. Building on it means starting at milestone 2 (the login gate).
One thing to know before touching it: `ClientEvent::Prompt` is new, because
the prompt used to be swallowed inside the client and the router had no
completion marker; it is marked `// DXCA:` in the Apache-2.0 Meridian module
like every other graft there. The router's `on_event` returns a `consumed`
flag, and **a consumed event must not flow onward** — that is what keeps
`sh/dx` output out of the spot pipeline.
Manoj wants to issue cluster commands through DXCA to the upstream nodes.
What remains after M3: only spotting (M4), which is refused by tier and
should stay that way until someone actually wants it — it is the one step
that transmits. The doc's load-bearing points, if you read nothing else: the
**login gate ships with the feature, not after** (7575 binds `0.0.0.0` with
no auth, and every node logs in as the shack callsign, so passthrough
without auth means the LAN can spot as VU2CPL); response correlation is
solved with a **per-node serialized command queue**, since the protocol has
no request IDs; and `SH/DX` output must never reach the spot pipeline or it
injects hours-old spots into everyone's feed and alerts. Four milestones,
and stopping after the third (read-only passthrough) is a fine place to
stop.

*(Resolved 2026-08-28: v2.2.2 is on both Pis. Adersh's went out in a second
pass once the VPN came back up — the subnet clash makes a two-host deploy
inherently two passes, which is the practical cost of that gotcha and worth
planning around rather than fighting.)*

*(Resolved 2026-08-28, same morning: the v2.2.1 loose ends are closed —
noderedpi4 confirmed 2.2.1 via `/api/status` after the VPN came down, and
the GitHub release is published with the Windows zip. The retry is live on
both Pis — see "Telegram sends retry once on transport errors".)*

**TODO (2026-08-28): MQTT publishes, but nothing shows on the panadapter.**
Manoj configured the `Shack` destination against `192.168.1.169:1883` as
`svc` and reports the publish counter climbing — so DXCA's half is
confirmed working end to end: connect, authenticate, publish. What has NOT
been shown to work is the **consumer** side, from the topics to a FlexRadio
/ Aether display. Deferred deliberately; not a DXCA bug as far as anything
observed so far.

**Narrowed the same evening — the broker side is RULED OUT.** A
`mosquitto_sub -t 'shack/dxca/spots/#'` as `svc` shows both topics carrying
live traffic, correctly formed. So publishing, authentication and the ACL
are all confirmed good, and the only missing piece is that **nothing
subscribes**: there is no Node-RED flow yet bridging
`shack/dxca/spots/json` to whatever Flex or Aether consume, and neither is
known to read MQTT natively. That bridge was always separate work, not part
of the DXCA feature — a point that should have been made plainer when MQTT
shipped.

That capture also validated three of the day's spot fixes on real traffic,
which is worth keeping:

- `RI1FJL` 21270.0 from **DB0SUE**, comment `QSX 21286.10 UP 16.10 LR40` —
  no mode word at all → `"mode":"SSB","mode_inferred":true`. The Region 3
  15m phone segment, honestly labelled. Before, this was blank and scored
  DATA.
- The **same station** from **N2WQ-2**, comment starting `USB …` →
  `"mode":"USB","mode_inferred":false`. The widened mode table; `USB` was
  absent from the 1.x list of ten.
- `4X6TU` **14100.0** from VU2OY, `9 dB 20 WPM` → `"mode":"CW"`, not
  inferred. That is the parser's WPM→CW read that `synthetic_spot` used to
  throw away — and 14.100 sits in the beacon window where band-plan
  inference deliberately declines, so without it that spot would have had
  no mode at all.

Topic shapes and payloads are in the "MQTT destinations" section below.

**Credential note (2026-08-28):** the `svc` broker password was pasted into
a Claude session transcript while debugging this. Rotate it in Mosquitto and
update the DXCA MQTT destination plus the other `svc` publishers
(`monitor.sh`, chrony, ubersdr — see `vu2cpl-shack/MQTT_AUTH.md`) if that
transcript is ever shared.

Nothing operational — **v2.1.0 is fully live**: ClubLog and Telegram are
configured and working on the Pi (confirmed by Manoj 2026-08-27), so
per-user highlighting and alerts run in production.

2026-08-27 (late): the deploy tooling was **generalized for third-party
installs** — dxca.service is a template (`__USER__` → the invoking user),
install.sh chowns to the invoker, and a fresh install self-bootstraps
(setup card, cty/LoTW download on demand).

> **Correction, same day:** that generalization was recorded as "validated
> by re-running the installer on the production Pi (identical result,
> service undisturbed)". The service being undisturbed was the
> `enable --now` bug, not a pass — the installer had replaced the binary
> and left the old process running. Genuinely validated now, twice: the
> restart fix on noderedpi4, and a real third-party install on
> adersh@192.168.1.151.

Remaining before any public release: x86-64-Linux release artifacts, then
the vu2cpl.com card with the VU3ESV credit line. (The repo-public flip is
DONE — `vu2cpl/dxca` is public and carries the `DXCA v2.0.0` release. The
Windows build test is DONE — see below.)

### Windows: builds, installs, runs (2026-08-28)

First Windows build and run in the project's history. **Zero source
changes were needed** — the `#[cfg(unix)]` gates on the SIGTERM handler
and the db `0600` chmod were the only Unix-specific code, and both
fall back correctly.

- **Build:** `just win` — `cargo zigbuild --release -p dxca-server
  --target x86_64-pc-windows-gnu`. A first attempt at
  `x86_64-pc-windows-msvc` failed in exactly two places, both C, neither
  ours: `ring` (`assert.h`) and `libsqlite3-sys` (`stdlib.h`), for want of
  Windows CRT headers on the Mac. zig bundles mingw-w64's, so the GNU
  target needs no Microsoft download or licence. **A native MSVC build is
  still untried.**
- **Bundle:** `just win-bundle` → `deploy/win-bundle.sh` produces
  `dxca-<version>-windows-x64.zip` (exe + installer + uninstaller +
  README-WINDOWS.txt + licence). It refuses to ship a binary carrying the
  placeholder page.
- **Verified on `manoj@192.168.1.170`** (DESKTOP-IP8PT88, Win10 22H2
  19045, AMD64): web GUI, `/api/*`, telnet banner, SQLite creation, boot
  -triggered LOCAL SYSTEM task, survives the installing session closing,
  firewall rule + LAN reach, clean uninstall, and an **update over an
  existing install that preserves `config\` and `data\`**.
- **Still unverified:** spot ingest (no decoder or cluster node has ever
  fed it), graceful shutdown (Ctrl-C-only path), long-run stability,
  Win11/Server/ARM64, MSVC.
- **The blocker to calling it *supported*** is unchanged: `data\dxca.db`
  holds ClubLog app passwords and Telegram tokens in plain text, and the
  `0600` protection is Unix-only. Windows needs DPAPI or an ACL hardening
  pass before this is more than "works".

Four Windows gotchas, each found by testing and now handled in
`deploy/windows/install-dxca.cmd` — worth reading before writing any
other Windows script here:

1. **Batch has no `\"` escape.** Shell-style escaping produced a broken
   path and the installer silently did nothing, exiting 1 with no output.
2. **`schtasks /tr` quoting breaks on paths with spaces**, and schtasks
   has no "start in". Fixed by generating a `run-dxca.cmd` wrapper and
   registering with PowerShell `Register-ScheduledTask -WorkingDirectory`.
   Without a working directory the relative `config\dxca.toml` never
   resolves and dxca silently runs on defaults.
3. **Firewall rules scoped `profile=private` are inert on a Public
   network** — present, enabled, and doing nothing while the server
   listens. Windows re-classifies on its own when an adapter
   re-identifies; `.170` flipped to Public mid-session. The installer now
   detects it and refuses to print a LAN URL that will not answer.
4. **`timeout /t` fails under non-interactive SSH** ("Input redirection is
   not supported") and does not wait at all. Use `powershell Start-Sleep`
   in anything driven remotely.

Also learned about the Meridian box while there, and **not yet fixed in
that repo**: `meridian/HANDOVER.md` claims a `meridian-webui` firewall
rule that does not exist; `MeridianServer` has a fixed `TimeTrigger`, not
a boot trigger, so it does not survive a reboot; and its task carries
`DisallowStartIfOnBatteries` + `StopIfGoingOnBatteries`.

**Open, small:**

- The local toolchain wart — `/usr/local/bin/cargo` shadows the rustup
  shims, so `just gate`'s lint step and all doctests fail for environmental
  reasons. Workaround recorded below; the fix is to remove the standalone
  Rust install or reorder PATH.
- The Spots screen's display narrowing is per-browser (`localStorage`), not
  per-account. PLAN's "own display filters" is only half done; server-side
  persistence was deliberately deferred to avoid a second setting to
  reconcile with My Alerts.
- `udp_sent` on the Pi sat at 0 for a while after a restart and the RUMlog
  destination is `192.168.10.226` while the shack LAN is `192.168.1.x`.
  It recovered (437 and climbing), so this is a "look again if
  click-to-fill misbehaves", not a known fault.

## DXCC Challenge points (2026-08-27)

On the Spots station card, beside DXCC. **A Challenge point is entity ×
band, mode-agnostic, over ten bands only** — 160/80/40/30/20/17/15/12/10/6.

Two things to keep straight, because both are easy to get wrong later:

- **60M does NOT score.** It is in `SELECTABLE_BANDS`, the resolver emits
  it, and the spots screen offers it as a filter — but a 60m QSL adds
  nothing to the Challenge total. The WARC bands (30/17/12) *do* score,
  which is the other half of the same confusion.
- **Challenge is not this crate's "slot".** A slot is band × MODE, so a
  station worked on 20M in both CW and FT8 is two slots but one Challenge
  point. The card shows both totals, side by side, for exactly that reason.

**Validated against ClubLog itself (2026-08-27):** VU2CPL's log —
56,815 QSOs, 320 DXCC worked / 319 confirmed, 4339 slots worked / 4075
confirmed — yields **2397 confirmed Challenge points, exactly what ClubLog
reports**. That single match covers a lot: the band table, the 60m
exclusion, the entity×band (not ×mode) rule, and `Record::is_confirmed` —
including its treatment of ClubLog's own `APP_CLUBLOG_QSO_QSL = Y` flag
alongside the three standard ADIF QSL fields, which was the part with no
independent reference. If the Challenge figure ever drifts from ClubLog's,
suspect `is_confirmed` first.

`bands::CHALLENGE_BANDS` / `is_challenge_band()`, summed in
`LogMatrix::stats()` into `challenge_worked` / `challenge_confirmed`. The
award counts the confirmed figure (1000 to claim, endorsements every 500);
worked is carried alongside because the gap is the QSL chase. The unit test
`challenge_counts_entity_bands_not_slots` pins the 60m exclusion and the
one-point-per-band rule together.

## The ClubLog API key is a SERVER setting (2026-08-27)

It used to sit in each user's ClubLog config. It never belonged there: the
key is only ever used for `cty.php`, which fetches **cty.xml** — one file
backing one shared `DxccResolver` that every account is classified against.
It is not, and never was, involved in downloading anyone's log; that uses
the operator's own email + app password.

Now symmetrical with the LoTW list, which had the same shape all along:

| | cty.xml | LoTW users |
|---|---|---|
| scope | server-wide | server-wide |
| credential | `Db::clublog_api_key` | none needed |
| refresh | admin only, `POST /api/cty/refresh` | admin only |
| schedule | `cty_refresh_days` (default 7) | `lotw_refresh_days` (default 7) |

**The key is in the DATABASE, not `config/dxca.toml`.** install.sh writes
the config file 0644 and `data/dxca.db` 0600 — putting a credential in the
TOML would have moved it somewhere *more* readable. Only the cadence, which
is not secret, is a file setting.

**`adopt_legacy_api_key`** lifts a pre-2.1 per-user key to the server
setting, once, at startup — so upgrading needs no manual step. It is guarded
by its own **ran-once flag**, not by "is the server key empty?". Those look
equivalent and are not: an admin who deliberately *clears* the key leaves it
empty, and an emptiness check would re-adopt the stale key from the user's
row on the next restart, silently undoing them forever. Test:
`legacy_per_user_api_key_is_adopted_once`.

A server with no key simply keeps the cty.xml it has; the scheduler stays
quiet rather than logging a failure every 15 minutes.

**DECIDED 2026-08-29 — DXCA will ship its own key.** See *NEXT: ship DXCA's
own ClubLog API key* under Open items for the four things that has to carry.
The deciding fact was not on this list: ClubLog issues keys to software
developers only, so the per-install field is one an ordinary operator has no
route to fill. The original reasoning, which still holds: the key is an
*application* credential, not a user one, so DXCA could ship a default. Two
caveats. Technically, an embedded key cannot be kept secret — the binary
must carry its own decryption key, and dxca passes it as a URL query
parameter, so tcpdump on the operator's own machine reveals it without
touching the binary. Treat any shipped key as public; don't build encryption
theatre. Practically, ask ClubLog (G7VJR) first: rate limits are per key, and
abuse by one installation would revoke it for all. If they decline, AD1C's
`cty.dat` needs no key but has no dated prefix windows or exact-call
exceptions, which `cty.rs` actively uses — a real downgrade.

## Automatic ClubLog / LoTW refresh (2026-08-27)

Both were manual-only until now — one button each — which on a 24/7 box
meant the log stopped moving whenever nobody pressed anything, and anything
worked since kept alerting as New DXCC. PLAN §5's "refresh schedule" line,
finally built.

- **Per-user ClubLog**: `refresh_hours` in the account's clublog config
  (0 = manual, default **24**). Set in the web UI, My ClubLog →
  Auto-refresh. Per-user because each account pulls its own log with its own
  credentials.
- **Server-wide LoTW**: `lotw_refresh_days` in `config/dxca.toml`
  (0 = off, default **7**). File-edited + restart, like the other scalars,
  and shown in System's file-only line. Server-wide because the list is one
  shared ~6 MB file.

`crates/dxca-server/src/refresh.rs`, spawned from main. Ticks every 15 min
and does **at most one job per tick** (LoTW first when both are due). Things
that are load-bearing:

- **Attempt stamps are separate from success stamps, and written before the
  outcome is known.** `matrices.last_refresh_unix` only advances on success,
  so a failing account would read as due on every tick and hammer ClubLog.
  `RETRY_AFTER_SECS` (1 h) is the floor either way, persisted so a crash
  loop can't reset it.
- **No refresh on boot.** The check is purely time-based; a restart pulls
  only what was already overdue.
- The LoTW **success stamp is written inside `UserService::refresh_lotw`**,
  not in the scheduler, so the manual button resets the automatic clock too.
- Timestamps live in a new **`meta`** table (`key`/`value`), *not* on file
  mtimes — `install -m 600` rewrites mtimes on every deploy and would reset
  the LoTW clock each time.
- The decision itself is a pure function, `is_due(now, last_ok,
  last_attempt, interval_secs)`, so it is unit-tested without SQLite or the
  network. Change scheduling behaviour there, not in the callers.

`ClubLogUserConfig::Default` is hand-written: deriving it would give a new
account 0 (manual) while serde's per-field default gives an existing stored
row 24, and those must agree.

## Alert levels 2.1 (2026-08-27)

**Four levels became eight, and the band/mode narrowing arrived on both
tabs.** The old `alert_unconfirmed` was a *global switch* that swapped the
whole comparison to the confirmed sets — so "never worked" and "worked but
unconfirmed" were mutually exclusive and the UI could never say which kind
of gap a spot was.

The ladder now runs, rarest first:

| | never worked | worked, not confirmed |
|---|---|---|
| entity | `newDXCC` | `unconfDXCC` |
| band | `newBand` | `unconfBand` |
| mode | `newMode` | `unconfMode` |
| slot | `newSlot` | `unconfSlot` |

`raw_level` decides the whole `New*` half against the **worked** sets first;
only a spot whose slot really is in the log reaches the **confirmed** sets.
That ordering is the meaning — a band never worked beats a band worked and
unconfirmed. `unconfDXCC` is checked before the narrower `?` levels because
with nothing confirmed for an entity the band/mode/slot gaps are all true.

`alert_unconfirmed` was **retired, not migrated**: serde ignores the leftover
key and the only stored account had it `false`. All four `?` levels default
off, so an existing account behaves exactly as it did until something is
ticked.

**Three narrowings, three scopes** — worth keeping straight, since "alerts"
now appears on three tabs:

- **My ClubLog** — which levels this account flags *at all* (the classifier's
  `alert_*`). Off here means the level never reaches the feed or Telegram.
- **Spots** — which flagged levels are on screen. Client-side, kept in
  `localStorage['dxca.spotfilter']`, deliberately NOT server-side: it is a
  per-browser view preference and persisting it would make it a second
  account setting to reconcile with My Alerts.
- **My Alerts** — which levels ping Telegram (`notify_*`), plus
  `notify_bands` / `notify_modes`. **Empty list = ALL**, the same convention
  `broadcast_destinations.sources` uses — which is why a fresh account is not
  silent.

New endpoints: `GET /api/reference` (bands, mode classes, the level ladder
with labels — served so the UI cannot drift from `AlertLevel`) and
`GET /api/me/station` (callsign + `MatrixStats` + QSO count for the Spots
station card). Confirmed-DXCC there follows the **award** rule: an entity
with at least one confirmed slot, not an entity fully confirmed.

Level colour is a CSS data table — `[data-level=…]` in app.css resolves
`--lvl` / `--lvl-bg`, and the feed row, Alert cell, chips and config lists
all read those two. A ninth level needs no CSS. The `?` half reuses its New
counterpart's hue pulled toward `--muted`, so hue says *which axis* and
saturation says *how badly you need it*.

`bands::SELECTABLE_BANDS` (160M–70CM) is narrower than `BANDS` on purpose:
LF/MF and microwave are still *resolved* from frequency, they just aren't
worth a checkbox here.

## DXSpider bells ate every spot (fixed 2026-08-27)

Adding **db0sue.de:8000** (DO5SSB-2, DXSpider 1.57) looked like a connection
failure. It wasn't: the node connected, logged in, and proved **Live** — and
then delivered nothing.

Its spot lines end `… 0508Z\x07\x07\r\n` — DXSpider rings the terminal on
every spot. BEL is not whitespace, so `str::trim` left it stuck to the last
token; `parse_spot_line`'s rightmost-`HHMMZ` search wants a 5-char token and
saw a 7-char `0508Z\x07\x07`, found no time, returned `None`, and every spot
fell through to the raw `Line` arm. **A node that reads healthy while
dropping 100% of its traffic** — worth remembering as a failure shape.

`wire::strip_c0_controls` now runs where lines are cut in `on_bytes` (tab
kept — it is real field whitespace), so the parse, the telnet fan-out and
the broadcaster all see the clean line. Two tests pin it, captured off the
wire; the wire-level one asserts the *un-stripped* line still fails to
parse, so the guard can't rot into a tautology.

DB0SUE is now the **fifth node** in `/opt/dxca/config/dxca.toml` (Live,
delivering). Config backup before the edit:
`/opt/dxca/config/dxca.toml.bak-before-db0sue`.

Note for triage: `/api/spots` fills `band` / `dxcc_name` / `alert` /
`is_beacon` **only for an authenticated session** (`annotate_spot`, api.rs)
— an unauthenticated `curl` shows them null and that is by design, not a
classification bug.

## Account editing and deletion (2026-08-27)

Accounts used to be **create-only**: `/api/users` was `get(list).post(create)`
and the db layer had no update or delete at all, so fixing a typo'd callsign
or removing a test account meant stopping the service and editing SQLite by
hand — with `PRAGMA foreign_keys = ON` typed manually, because the CLI
defaults it off and would otherwise orphan the user's config and matrix.

Now `PATCH /api/users/{id}` and `DELETE /api/users/{id}`, both admin-gated,
with Edit/Delete buttons per row in the Users tab. PATCH takes any subset of
callsign / display_name / role / password; absent fields are left alone, so
the UI sends only what changed and an untouched password box is not an empty
string. Callsign is uppercased on write exactly as `create_user` does —
otherwise a lowercase rename would produce a row that `user_by_callsign`
(which uppercases its argument) could never match, i.e. an account nobody
can log into. A rename onto a taken callsign is checked *before* any write,
so the operator sees "already exists" rather than a raw UNIQUE-constraint
string.

**The guard rule, and why it is asymmetric.** Deleting the last account is
ALLOWED — the roster goes to zero, `/api/setup` re-arms, and that is the
intended way to start a server over. What is refused is removing *or
demoting* the last **admin** while other accounts remain: `/api/setup` only
opens at zero accounts, so that state leaves users nobody can administer and
no route back through the web UI at all. Demotion is refused regardless of
the account count, since unlike deletion it can never reach zero.

Deleting your own account is allowed and is not a special case — sessions
cascade with the row, so the cookie dies with it; the UI reloads into the
login (or setup) card. `tests/user_admin.rs` walks the whole thing end to
end, including the dead-cookie assertion after an admin deletes themselves.

Not done: no audit trail of who changed what, and no confirmation step
beyond the browser `confirm()`. Both were judged out of proportion to a
shack-scale roster of two or three accounts.

## v2.2.0 (2026-08-28)

**Windows.** dxca builds, installs and runs on Windows for the first time —
no source changes required. Ships as `dxca-2.2.0-windows-x64.zip`: a
self-contained `.exe`, an installer that registers a boot-triggered LOCAL
SYSTEM task and optionally opens the firewall, an uninstaller, and the
disclaimers. Full detail, the four batch/Windows traps it encodes, and the
list of what remains unverified are in the Windows section above.

Also in this release, previously carried by commits after the `v2.1.1` tag
and therefore never in a tagged build: MQTT spot publishing for panadapter
overlays, the server-wide call blacklist, alerts-sent history including
failures, ClubLog log statistics, the boxed status bar and chip-row Sources
filter, and the installer's behind-the-remote warning. The version string
now matches a tag containing all of it.

## v2.1.1 (2026-08-28)

Seven commits on top of v2.1.0, in three groups. Each has its own section
below with the reasoning; this is the index.

| what | why it exists |
|---|---|
| Spot-mode inference + no more silent DATA | an unknown mode was being scored into the operator's digital award slots |
| Account edit + delete (`PATCH`/`DELETE /api/users/{id}`) | accounts were create-only; fixing a callsign meant hand-editing SQLite |
| Installer: rustc gate, Node gate, real rebuild, self-verification | the VU2WJ install failed four different ways, each reporting success |

**Deployed to noderedpi4 2026-08-28 01:10 IST** with
`deploy/pi-deploy.sh --no-seed vu2cpl@noderedpi4.local`. `--no-seed` on our
own Pi too: the box already has its config and database, and the flag stops
this Mac's `dxca.db` being copied into `~/dxca-deploy/` for nothing. The
installer's own check confirmed the dashboard was serving before it exited.

Verified against live traffic straight after: 38 spots in the ring, **0 with
a blank mode**, 2 inferred — both from DB0SUE, both correct (3.749 MHz →
SSB, 7.020 MHz → CW). Small sample; the ring is in-memory and the restart
emptied it. Worth re-checking on a full ring:

```sh
ssh vu2cpl@noderedpi4.local 'curl -s "http://127.0.0.1:7580/api/spots?limit=2000"' \
  | python3 -c "import json,sys; s=json.load(sys.stdin)['spots']; print(sum(1 for x in s if not (x.get('mode') or '').strip()), 'blank of', len(s))"
```

## Missing mode on cluster spots — and the DATA default behind it (2026-08-28)

Reported as "mode is missing in some of the spots, noticed N2WQ", then
narrowed to **DB0SUE and N2WQ**. Both relay *human* spots, whose comment is
free text with no mode field, so nothing in the pipeline could name a mode.
Chasing it turned up four separate defects, the last of which was the one
that mattered.

**1. The parser's own answer was thrown away.** `wire.rs` parses mode from
comment *tokens* and additionally infers CW from a `WPM` token and RTTY from
`BPS`. `synthetic_spot` ignored `p.mode` entirely and re-scanned the comment
itself. A skimmer line commented `-15 dB 22 WPM` therefore arrived with no
mode even though the parser had already worked out CW.

**2. Substring matching invented modes.** The 1.x scan was
`comment.contains("CW")`, so `QSL via N1CW` scored CW, `tnx OM DO5SSB relay`
scored SSB, and `CWops number 123` scored CW. A wrong mode is worse than a
blank one: it files the spot in an award slot it does not belong to, and
nothing ever flags it again.

**3. The known list was ten modes and had no `USB`/`LSB`.** An ordinary
phone spot commented "USB" got no mode while the identical spot commented
"SSB" got one. `JS8`, `Q65`, `FST4`, `PSK63`, `OLIVIA`, `FM`, `SSTV` were
all likewise invisible.

**4. An unknown mode was silently scored as DATA.** This is the real bug.
`modes::canonical("")` returns `"DATA"`, and `classify` fed it straight into
the award ladder — so a 14.200 phone spot with no mode was credited to the
operator's **digital** slots, capable of firing a false New Slot/New Mode
alert and of masking a genuine phone need. Nothing about that was visible.

### What it does now

Mode is settled in three steps, best source first: the parser's token-based
`p.mode`; then a widened, **token-matched** comment scrape; then, only if the
spot genuinely says nothing, `bands::mode_from_mhz`.

The band plan is **IARU Region 3** by explicit choice (this shack's own).
Digital watering holes (`14.074` FT8, `7.0475` FT4, JS8, WSPR…) are checked
*before* the broad segments, because several sit inside a phone segment —
50.313 FT8 is in the middle of the 6m SSB range and would otherwise infer as
SSB. Segments deliberately leave gaps (beacon windows, 60m, everything above
2m): an uncertain frequency infers **nothing** rather than something wrong.

Every inferred mode is marked. `Spot::mode_inferred` rides through the API
and the WebSocket frame, and the Spots table underlines an inferred mode with
a dotted rule and a tooltip; a mode that could not be inferred at all shows
`—`. The operator can see which award slots rest on a guess.

`classify` no longer bottoms out at DATA. `modes::canonical_opt` returns
`None` for an unknown mode and `raw_level` then answers **only the band half
of the ladder** — New DXCC and New Band still report, because those are
mode-independent, while New Mode and New Slot are withheld rather than
invented. The web UI's `modeClass` mirrors the same rule, so an unknown-mode
spot matches no mode narrowing instead of hiding behind the DATA chip.

### Honest limitation

A spot's mode follows the **transmitting** station's band plan, not ours. A
Region 1 station working phone low in 40m can infer wrongly under a Region 3
table. That is why the segments are coarse and why inference is labelled
rather than asserted. If this proves annoying, the options are a
per-region table keyed on the spotted call's DXCC, or dropping segment
inference and keeping only the watering holes.

**Test gotcha worth remembering:** the ±500 Hz watering-hole tolerance is
compared in **integer Hz**. As MHz f64, `(14.0745 - 14.074).abs()` is
0.0005000000000000004 — a dial exactly 500 Hz up fell outside its own
tolerance. The test caught it; the float version would have shipped.

## install.sh now verifies its own work (2026-08-27)

Every failure in this script's history looked like a **successful** install:
the unit started, the URL printed, the script exited 0, and the dashboard
was a placeholder. Nothing in the installer ever looked at the result. So it
now finishes by fetching its own web page:

- **Pi/Linux**: `systemctl is-active` first — a unit that failed to start
  gives a far better error than a connection refused twenty seconds later —
  then the HTTP check, then the LAN URL.
- **macOS**: `launchctl print` on the agent, then the same HTTP check.
- Both poll for up to 20s (a fresh service needs a moment to bind; failing
  on the first refused connection would be a false alarm), then look for
  build.rs's `Web UI not built into this binary` marker. Finding it is a
  hard failure — **unless `--stub-ui` was passed**, in which case the
  placeholder is what was asked for and the check passes.
- `web_url` reads `web_bind` out of the installed `dxca.toml`, so a
  non-default port is probed correctly. A wildcard bind (`0.0.0.0`, `[::]`)
  is probed on loopback; a specific address is used as-is, since loopback
  would not be listening then. No config at all falls back to 7580.
- No curl on the box is a skip-with-a-note, not a failure.

Tested against fixture servers: real dashboard passes, placeholder fails,
placeholder + `--stub-ui` passes, nothing listening fails with the right
log command for the platform. `web_url` verified across all five bind forms.

**Follow-up, same day — the Node version check landed too**, closing the
asymmetry with the rustc gate. `node_gate` runs before `pnpm install`.

The rule is **not a plain floor**, which is the whole trap: vite 6 and
`@sveltejs/vite-plugin-svelte` 5 both declare `engines.node` as
`^18 || ^20 || >=22`, so the odd-numbered non-LTS releases **19 and 21 are
excluded even though they are newer than 18**. `>= 18` would wave them
through; `>= 20` would reject a working 18. `NODE_ENGINES` records the
string and the comment says to re-read those two `engines` fields before
touching it — they are the source of truth, not us.

`--stub-ui` skips the dashboard build rather than dying, same as for a
missing pnpm. A `node --version` that returns nothing is a note, not a
failure: pnpm is itself a Node program, so that state is near-impossible
and not worth failing an install over.

Tested against stub `node` binaries at 16 / 18 / 19 / 20 / 21 / 22 / 24 / 26
— accepted, rejected, and the `--stub-ui` skip all behave.

## My Alerts shows what was actually sent (2026-08-28)

Requested as "in alerts tab, alerts sent should be shown like spots list".
The fan-out is fire-and-forget on a background thread, so from the UI a spot
that was **flagged**, **narrowed away** by the band/mode chips, **held by the
cooldown**, or **refused by Telegram** all looked identical: nothing arrived.
That was unanswerable, and it is what this fixes.

New `alerts_sent` table (per user, `ON DELETE CASCADE`, indexed on
`user_id, time_unix DESC`) written by `fan_out` **after** the send, with its
verdict. `GET /api/me/alerts` serves the caller's own; the My Alerts tab
renders them in the Spots row vocabulary — level tint via `[data-level]`,
same columns — and re-polls every 15 s, because a history that only updated
on reload would be the same invisibility again.

**Failures are recorded, with Telegram's own error text**, and shown as a
`failed` chip whose tooltip is the reason. A "sent" log that stored only
successes would hide the single most useful row on the page — the shack
broker analogue is a bad chat id, which otherwise fails in silence forever.

> **Correction (2026-08-28): the level tint above was claimed, not
> delivered.** The rows carried `data-level` from the start, and this doc
> said they rendered "in the Spots row vocabulary — level tint via
> `[data-level]`" — but no rule in `Alerts.svelte` ever *painted* with the
> `--lvl` / `--lvl-bg` those attributes resolve. Dashboard's two painting
> rules (`tr.flagged td`, `tr.flagged .alert`) are scoped to Dashboard, as
> Svelte scopes all component CSS, so My Alerts showed uniformly grey rows.
> Fixed by adding the pair to `Alerts.svelte`, keyed on `tr[data-level]`
> rather than a `.flagged` class — every sent alert was flagged by
> definition, so there is nothing to gate on. Verified across all eight
> levels in both themes: each resolves a distinct wash and Level colour,
> `?` variants reading as muted versions of their `New` counterparts.
> Worth generalizing from: `data-level` on an element buys nothing on its
> own, and a third table wanting the tint will need these two rules again
> (or app.css would have to paint, which it deliberately does not).

Bounded at `ALERT_HISTORY_MAX = 500` **per user**, pruned on insert, so a
busy operator cannot evict another account's history. That is asserted, not
assumed: the unit test floods A past the cap and checks B's single row
survives.

Test coverage, honestly: `users_alerts.rs` proves the delivered path end to
end through the real fan-out — one row for B, `newDXCC`, `delivered: true`,
band and entity carried, **A's history empty** though both users saw the
same spot, and a 401 without a session. The **failure** path is covered at
the storage layer (`db.rs`, delivered=false with its error round-tripping)
rather than end to end, because the fake Telegram in that test always
answers 200 and the cooldown blocks a second alert for the same call.

## Telegram sends retry once on transport errors (2026-08-28)

Prompted by a field report: Adersh's screenshot of his My Alerts page with
red `failed` chips, "why failed?". The `alerts_sent.error` column on his Pi
(`adersh@192.168.1.151`, over the VPN) answered it — 8 of 41 alerts failed
overnight (~03:00–06:20 IST), all with one of two errors, both transport:

- `tls connection init failed: Resource temporarily unavailable (os error 11)`
  — the TLS handshake to `api.telegram.org` timed out mid-setup;
- `Network Error: timed out reading response` — connected, but no reply
  within the sender's 10 s limit.

Not a config problem: token and chat id are fine (those would fail as HTTP
4xx from Telegram), successes interleaved with the failures, and at test
time his Pi reached `api.telegram.org` in ~0.8 s consistently (IPv6,
~280 ms RTT). Classic night-time congestion blips on a residential uplink —
and with the old single-attempt sender, each blip was a lost alert.

**Fix:** `Telegram::send` now makes **one retry after 2 s, on transport
errors only**. An HTTP rejection (bad token, unknown chat) still returns
immediately — Telegram would only refuse it again, and retrying those would
double-send nothing while masking real misconfiguration. Both call sites
(`fan_out`, the test button) already run `send` under `spawn_blocking`, so
the pause cannot stall the pipeline. When the retry also fails, the recorded
error says so: `… (retried; first attempt: …)` — the My Alerts tooltip then
shows both verdicts. `retry_delay` is a public field (default 2 s) zeroed in
tests.

Tests (`telegram.rs`): a local TCP stub whose per-request closure either
answers or drops the connection. `transport_error_is_retried_once` — first
connection dropped, second answered 200, exactly 2 hits.
`http_rejection_is_not_retried` — always 400, exactly 1 hit. Stub gotcha
worth keeping: the stub must read the **full request body before replying**,
else the client's body write fails and a 400 test reads as a transport error
(that false start cost one red test run).

**Deploy status: SHIPPED as v2.2.1**, tagged and deployed to both Pis the
same morning via `pi-deploy.sh` (adersh with `--no-seed`). Adersh's Pi
verified end to end: `/api/status` reports 2.2.1, service active, cluster
nodes reconnected, his account untouched. noderedpi4's installer
serving-check passed immediately; its `/api/status` read confirmed 2.2.1
once the VPN came down (the subnet-overlap gotcha — see Known gotchas —
had cut the Mac off from the shack LAN mid-verification). GitHub release
published: https://github.com/vu2cpl/dxca/releases/tag/v2.2.1.

## My ClubLog shows the log's statistics (2026-08-28)

Requested as "My ClubLog after a refresh should show all statistics for that
user". It previously reported a refresh as one sentence — *"Refreshed: 56816
QSOs, 320 DXCC entities"* — which scrolled away and told you nothing about
the log itself.

There is now a **Log statistics** card: QSO count, log callsign and refresh
age; the six award totals (DXCC / Challenge / Slots, worked beside
confirmed); and, new, **entities per band** and **entities per mode**.

`LogMatrix::by_band_and_mode` slices the same in-memory matrix `stats()`
already walks, so this costs one pass over the entity map and **no new
storage or endpoint** — it rides `/api/me/station`, the endpoint the Spots
station card uses, so the two screens can never disagree about the log.

Deliberate choices:

- **Entities, not QSOs.** A band's figure is how many DXCC entities have at
  least one contact there, which is what the award counts. Stated on the
  card, because "20M: 3" is otherwise ambiguous.
- **Empty rows are kept**, dimmed rather than hidden. A band with nothing on
  it is the most useful row on the page.
- Ordering is `SELECTABLE_BANDS` (160M first) and `modes::CLASSES`
  (CW/PHONE/DATA), not hash-map order.
- `refresh()` reloads the card, which is the actual ask — a refresh has to
  become visible as numbers.

**Gotcha that cost a round trip:** Svelte scopes component styles, so
System.svelte's `.stats` block does **not** reach ClubLog.svelte. The six
totals rendered stacked one per line until an identical block was added
locally. Same shape on purpose — the two screens report the same numbers and
should not look like different things.

Verified by seeding a synthetic matrix straight into the `matrices` table
(four entities across 80/40/20/15/10M, partially confirmed) and reading the
rendered table back: 20M 3 worked / 2 confirmed, 40M 2 / 2, 15M 2 / 0, 10M
1 / 0 — matching the seed exactly.

## System-tab editors dragged the page sideways (fixed 2026-08-28)

Reported against the new MQTT card. Two faults, one visual and one that
looked like a data problem.

**The row is eleven columns** — name, broker, port, user, password, base
topic, client id, sources CSV, plus two checkboxes and the delete — which at
the editor's input widths comes to roughly **77rem (~1230px)**. Wider than a
1280px laptop, so the whole page scrolled horizontally and the nav slid off
the left. Every editor table is now wrapped in `.editor-scroll`
(`overflow-x: auto`), so a wide row scrolls **inside its card** and the page
never does. Measured after the fix at a 1350px viewport: page
`scrollWidth == clientWidth`, table 1240px inside a 974px wrapper that
scrolls.

**"published 0, failed 0" never moved.** The counters were only fetched on
mount and after a save, so they sat at zero while spots were in fact being
published, until a reload. They are polled on the same 5 s tick as the
server status now — via a **stats-only** `loadMqttStats()`, deliberately
separate from `loadMqtt()`: the latter replaces the `mqtt` array, which is
bound to the inputs, so polling it would have wiped whatever the operator
was halfway through typing.

Gotcha for the next person doing a bulk edit here: a `perl -0pi` replacing
`      </table>` also matched the **refdata** and **nodes-live** tables,
which are not editors, and produced two stray `</div>`s. The Svelte compiler
caught both. Wrapping by opening tag is safe; closing tags at a shared
indentation are not.

## MQTT destinations (2026-08-28)

Requested as "add an MQTT send option to destinations like FlexRadio or
Aether to display on the panadapter; setup should include broker, port,
auth". Manoj chose **both payloads** (JSON and cluster line, sibling topics)
and **one configurable base topic**, default `shack/dxca/spots` per the
shack's `shack/<service>/…` convention.

**A sibling of `broadcast.rs`, not a variant of it.** A UDP destination is a
datagram to an address; an MQTT destination is a connection with
credentials, keepalive and reconnect. Folding them into one struct would
have meant a dummy IP on every MQTT row and a dummy topic on every UDP one,
so `crates/dxca-connect/src/mqtt.rs` is its own module and MQTT rows are
their own list.

**Stored in the database, not `config/dxca.toml`.** The broker password is a
secret and that file is installed 0644 while `data/dxca.db` is 0600 —
exactly the reasoning that moved the ClubLog API key. Kept as one JSON blob
under the `mqtt_destinations` meta key: a short list edited as a whole.
Consequence: the MQTT editor in the System tab has its **own** Apply & save
button, because it does not go through `/api/config/global`.

**Dependency added: `rumqttc` 0.25, `default-features = false`** — that drops
TLS, websocket and proxy, none of which the shack broker uses. Turning
`use-rustls` back on is the change to make if a broker ever needs 8883. The
1.88 rustc floor did not move.

Notes:

- `try_publish`, not `publish`, and QoS 0: a spot feed is a live stream, so
  dropping when the outbound queue is full is right and blocking the
  pipeline on a slow broker is not. Drops are counted per destination.
- The rumqttc event loop **must** be driven or nothing is ever sent — one
  named thread per destination drains `connection.iter()`. Errors there are
  the reconnect path, not a reason to stop.
- `apply_mqtt` replaces the whole publisher, so dropping it closes the old
  connections and their threads: an edited broker address really is the one
  in use.
- Band is derived in the payload rather than carried — every consumer wants
  it and only the frequency is authoritative. Off-band frequencies publish
  `"band": null` rather than guessing.
- The same `sources` allowlist and `unfiltered` flag as UDP, and the same
  dedupe verdict, so MQTT and the logger see one consistent feed.

`tests/mqtt_publish.rs` stands up a **minimal MQTT 3.1.1 broker** — CONNECT
/ CONNACK, QoS-0 PUBLISH, PINGREQ, and nothing else — points a destination
at it through the HTTP API, pushes a spot through the real pipeline, and
reads what actually arrived on the socket: username and password on the
wire, both topics, the JSON's derived band, the cluster line, the
trailing-slash guard (`spots/` must not yield `spots//json`), and that
disabling a row stops publishing. Asserting on a config round-trip and
trusting the library would have proved none of that.

## Blacklist tab (2026-08-28)

Requested as "add a tab for blacklisted calls". Three decisions, all Manoj's:

**Server-wide, admin-managed** — one list, not per-user. The first pass at
the question offered scope and effect separately and allowed an impossible
pairing (per-user + drop-at-pipeline); the ring is shared, so a pipeline drop
cannot be per-user. Re-asked as one coupled choice.

**Drops at the pipeline**, before the ring: gone from the Spots table, the
telnet cluster server, the filtered UDP destinations and Telegram, for
everyone. Not a display filter.

**Exact match on the spotted DX call**, case-insensitive, no wildcards.

Implementation notes:

- New `blacklist` table (`callsign` PK, `added_unix`). `CREATE TABLE IF NOT
  EXISTS`, so an existing database picks it up with no migration step.
- The pipeline holds its own `RwLock<HashSet<String>>` rather than querying
  SQLite per spot — this is the hot path for every decode and every cluster
  line. `apply_blacklist` swaps it, the same hot-apply shape as sources and
  destinations, so an edit lands on the next spot with no restart. `main`
  loads the stored list before the first spot can arrive.
- The check sits **after** the `source_counts` increment on purpose: that
  counter is what proves a node is alive, and a node sending only blocked
  calls is still up. The count means "received", not "shown".
- `GET/POST /api/blacklist`, `DELETE /api/blacklist/{callsign}`, all
  admin-gated. Every write refreshes the live set as well as the database.

**Known limitation, stated in the UI and the README:** the verbatim UDP
passthrough forwards decoder datagrams *before* parsing (that is what makes
click-to-fill work), so a blocked call inside a WSJT-X decode still reaches a
logger by that path. Cluster spots have no passthrough and are dropped
completely. Closing that would mean parsing before passthrough and giving up
1.x byte-verbatim parity — deliberately not done.

`tests/blacklist.rs` drives the real API and the real pipeline: baseline
through, block, next spot never reaches `/api/spots` while the pre-block one
survives, idempotent re-add, unblock, spot flows again, 404 on removing
something unlisted, 401 for anonymous.

## Status bar boxed by category; Sources became a chip row (2026-08-28)

Two reports on the Spots screen, both fixed together because they are the
same strip of UI.

**The status pills were one long horizontal line**, and every cluster node
appeared in it **twice** with identical counts — `DB0SUE 110` and
`DB0SUE 110 10s`. Not two bugs: `process_spot` increments `source_counts`
for *every* spot, so a cluster node lands in `spots_per_source` as well as
in `cluster_nodes`, and the flat row rendered both maps end to end. It read
as duplication because it was.

Now four labelled boxes — **Decoders**, **Cluster nodes**, **Feeds out**,
**Reference** — sized to their contents and wrapping, the Meridian shape.
Decoders is the sources that are *not* nodes, so a node's count appears once
in the box that also carries its state and age. An idle decoder list says
"nothing decoding" rather than vanishing, because a decoder that has stopped
feeding is exactly what you want to notice.

**Sources was a `<details>` checkbox dropdown** while every other narrowing
on the screen was a chip row, so it hid both what was available and what was
picked. It is the same `ChipGroup` as Alerts / Modes / Bands now: All, then
one chip per source, empty set meaning everything. The bespoke `toggle()`
helper, the `.menu` / `summary` / `details` CSS and the `.fsep` separator all
went with it — the build is warning-free rather than carrying dead selectors.

Verified on a real instance rather than by eye: a throwaway server pointed at
noderedpi4's own telnet feed on 7575, so real spots populated both maps and
the duplication case actually occurred.

## "CQ only" filtered nothing (fixed 2026-08-28)

Reported as "cq only has no effect". Measured on production before touching
anything: **800 of 800 spots started with "CQ "**, across all five sources.
The checkbox was wired correctly; it had nothing to bite on.

`synthetic_spot` built every cluster spot as `message: format!("CQ {}",
p.call)`, and both `Spot::is_cq()` and the UI filter tested that string. So
the answer was yes by construction. Meanwhile `wire.rs` had already worked
out the real `SpotKind` (Cq / Dx / De / Bcn / Ncdxf, `Unknown` when the
comment carries no marker) and `synthetic_spot` discarded it — the same
shape as the mode bug two sections down: real information thrown away and a
confident-looking default put in its place.

`is_cq` is now a **stored field** on `Spot`, not a derivation:

- cluster: `SpotKind::Cq | SpotKind::Dx`, **or the spotter is a skimmer**.
  Manoj chose the skimmer widening deliberately — a skimmer only reports
  stations calling CQ, so an unmarked skimmer spot is one even though its
  comment never says so, whereas an unmarked human spot is somebody logging
  a station they heard. Strict marker-only was the alternative and would
  have hidden most of the feed behind the filter.
- decoder: `message_is_cq()` on the real decoded text, where the prefix
  genuinely means something.

`message` deliberately stays `"CQ <call>"` whatever the kind, because
`dx_callsign()` parses the callsign out of it and both the outbound cluster
line (`format.rs`) and `duplicate_key` ride on that. The spotter's actual
text is carried in the new `Spot::comment` and is what the Spots table now
shows in the Message column — previously it displayed a synthesised "CQ
<call>" for every cluster spot, which was never what anyone typed.

## Users edit row overflowed the card (fixed 2026-08-28)

Reported with screenshots: editing a user at a wide window pushed the Save
/ Cancel buttons and the password field out of the USERS card and across the
ADD USER card beside it. Narrow windows were fine, which is the clue.

`.card-grid` is `columns: 26rem` — CSS multi-column, so each card sits in a
fixed ~416px track. The edit row put three `<input>`s and two buttons into
table cells, and `td { white-space: nowrap }` means those cells cannot
shrink: the table's min-content width was far wider than the track, so it
spilled into the neighbouring column. A narrow viewport collapses to a
single column as wide as the page, which is why it only showed up wide.

Setting `width: 100%` on the inputs would not have fixed it — an `<input>`
carries an intrinsic min-content width from its default `size`, and table
layout honours that. Inline inputs were never going to fit a 26rem track.

The edit form now spans all four columns in one `<td colspan="4">` and uses
the same `.settings-form` label/field grid as the Add-user card, so it
sidesteps the table's intrinsic sizing entirely and matches the existing
visual vocabulary. `.edit-row td` is the one place the table is allowed
`white-space: normal`; roster rows stay nowrap.

Verified against a real instance rather than by eye: a throwaway server on
127.0.0.1:7599 with two seeded accounts, driven in the browser at 900,
1199 and 1500 px — no overflow at any of them — plus a save round-trip
confirming the restructured form still writes (roster updated, "Updated
VU2CPL." shown).

## install.sh does not pull — but it says when you should have (2026-08-28)

Asked directly: "will it pull latest in the install script?" It will not, and
should not. `install.sh` runs no git command that changes anything; it builds
the working tree as it stands. Pulling would be deciding about someone else's
code — fighting local edits, tripping over a detached HEAD — and it cannot
work at all from a `pi-deploy.sh` bundle, which has no repo.

The hazard is the other half: re-running the installer *without* pulling
rebuilds the OLD tree and reports success, which is indistinguishable from a
working update. That is the same shape as every other bug in this file.

`git_currency_note` therefore refreshes the remote-tracking ref only —
inside `.git`; the working tree and checked-out commit are untouched — and
prints how many commits behind upstream the tree is, before the build
starts rather than after ten minutes of compiling. It is a **NOTE, never a
stop**: installing an older checkout is a legitimate thing to do (a
rollback, a branch under test), unlike a missing dashboard.

Silent when the tree is current, when there is no upstream, and when there
is no `.git`. `GIT_TERMINAL_PROMPT=0` so a repo wanting credentials fails
rather than hanging an unattended install, and an unreachable remote says
the currency is *unknown* instead of implying the tree is current. Tested
across all five shapes: current, five behind, detached HEAD, unreachable
remote, no repo.

## install.sh did not install the web GUI (fixed 2026-08-27)

Two separate holes, both ending in the same symptom: the service comes up,
the dashboard does not. What you get instead is build.rs's placeholder —
*"Web UI not built into this binary"* — which is easy to read as a broken
install rather than a missing build step.

**1. A re-run never rebuilt.** The pi/linux branch picked the first of
`./dxca` or `target/release/dxca` that existed and, if it found one,
skipped the entire `require_cargo` / `build_web` / `cargo build` block. So
the classic recovery — hit the missing-pnpm warning, install pnpm, re-run
`./install.sh` — reused the stale binary and kept serving the placeholder
**forever**. Nothing in the output said so; the install "succeeded" each
time.

The two cases were being conflated. A git clone has `crates/` and must
always rebuild (cargo is incremental, so an unchanged tree is cheap). A
`pi-deploy.sh` bundle has no `crates/` and no `web-ui/` — just a `./dxca`
cross-compiled on the Mac with the dashboard already inside. The branch now
keys on `[ -d "$REPO/crates" ]`, and a directory with neither is a `die`
instead of a confusing half-install.

**2. Missing pnpm was a warning.** `build_web` printed a NOTE and carried
on, which is right for `cargo build` (the Meridian rule: plain builds never
need Node) but wrong for an installer — the web GUI is part of what
"install" means. It is now a hard stop naming the platform's install
command, with `--stub-ui` as the explicit opt-out for a deliberately
headless install.

`--stub-ui` meant install.sh needed real argument parsing, so it now loops
over `"$@"` like pi-deploy.sh does, and `--help` prints the header comment
via awk rather than a hardcoded `sed` line range — the range had already
silently truncated the help by the time it was first tried.

Verified with stubbed toolchains: build_web hard-stops on both platforms
with the right hint, `--stub-ui` proceeds, and the branch picks rebuild /
prebuilt / die for a clone-with-stale-binary, a deploy bundle, and an empty
directory respectively.

**Follow-up — never suggest `apt install nodejs npm`.** The first version of
that hard-stop message did, and on VU2WJ's Pi it failed outright: Node there
came from **NodeSource** (22.23.2), whose `nodejs` package provides its own
npm and declares `Conflicts: npm`, so apt refused with ~30 unsatisfiable
`node-*` dependencies. The fix on that box was simply `sudo npm install -g
pnpm` — npm was already present the whole time.

`build_web` now prints the command that fits the box it is running on:
npm present → `npm install -g pnpm`; only corepack → `corepack enable
pnpm`; macOS → `brew install node pnpm`; nothing → `apt install -y nodejs`
(**without** `npm`) then npm's own pnpm. Tested against stub PATHs for all
four shapes.

## The rustc floor is 1.88, and install.sh now enforces it (2026-08-27)

A third-party install on VU2WJ's Pi died at `cargo build` with *"rustc
1.85.0 is not supported by the following packages"* — twelve `icu_*` /
`idna_adapter` crates wanting 1.88 or 1.86. None of them are ours:

```
ureq 2.12.1 -> url 2.5.8 -> idna 1.1.0 -> idna_adapter 1.2.2 -> icu_* 2.3.0
```

The floor is therefore set by the committed `Cargo.lock`, and no manifest
in the workspace declares a `rust-version`, so cargo only complained deep
in dependency resolution — minutes in, after downloading 148 crates.

Two things made this land on a *fresh* box and never here: Debian Trixie's
`apt install cargo` gives exactly **1.85.0**, and a distro rustc ignores
`rust-toolchain.toml` (`channel = "stable"`), so it never self-corrects.
This Mac has been on 1.96.1 throughout.

`install.sh`'s `require_cargo` now checks `rustc --version` against
`MIN_RUSTC=1.88` before any build, and branches the remedy on whether
rustup is present — stale toolchain (`rustup update stable`) versus distro
package (install rustup, then confirm `which rustc` is `~/.cargo/bin`).
Comparison is major.minor via awk, so `1.99.0-nightly` passes. Verified
under `/bin/bash` 3.2 against fake toolchains at 1.85.0 / 1.88.0 / 1.96.1 /
1.99.0-nightly / 2.0.0, plus no-cargo and cargo-without-rustc.

**Follow-up, same day — `rust-version = "1.88"` is now declared** in
`[workspace.package]` and inherited by all three crates via
`rust-version.workspace = true`. Cargo now refuses in seconds with its own
message, and because the workspace is on `resolver = "3"` (MSRV-aware) a
future `cargo update` will prefer dependency versions that keep the floor
where it is instead of raising it silently. Adding it left `Cargo.lock`
untouched and `cargo check --workspace --all-targets` clean.

**There are now two constants, and they must move together:**
`rust-version` in `Cargo.toml` and `MIN_RUSTC` in `install.sh`. The
installer's check was kept on purpose — it fires before the pnpm web build
and before the first sudo, and it can name the remedy (stale rustup versus a
distro package that ignores `rust-toolchain.toml`), which cargo cannot.

Note the floor is the *lockfile's*, not the edition's: edition 2024 only
needs 1.85. If a dependency bump raises the real floor, both constants move.

## Local toolchain wart (2026-08-27)

`/usr/local/bin/cargo` + `/usr/local/bin/rustc` (a standalone Rust install)
**shadow the rustup shims** on this Mac, and that install ships no
`cargo-fmt`, `cargo-clippy`, or `rustdoc`. So `just gate`'s lint step and
`cargo test`'s doctests both die with "no such command" / "could not execute
rustdoc" — nothing to do with the code. Run the gate through the toolchain's
own bin dir until it's sorted:

```sh
TC=~/.rustup/toolchains/stable-aarch64-apple-darwin
PATH="$TC/bin:$PATH" "$TC/bin/cargo" fmt --all --check
PATH="$TC/bin:$PATH" "$TC/bin/cargo" clippy --workspace --all-targets -- -D warnings
```

Real fix when there's time: remove the standalone install so rustup's shims
win (`/usr/local/bin/{cargo,rustc,...}`), or put `~/.cargo/bin` ahead of
`/usr/local/bin` on PATH.

## Shell gotcha: never put a non-ASCII byte after `$VAR` (2026-08-27)

`echo "Shipping to $HOST…"` died with **`HOST?: unbound variable`** the
first time pi-deploy.sh was run from Manoj's own terminal. Not the ellipsis
being unprintable — bash 3.2 (macOS `/bin/bash`) and any non-UTF-8 locale
treat the ellipsis's high bytes as *identifier* characters, so the variable
actually looked up was `HOST\xe2\x80\xa6`. Under `set -u` that is fatal, and
the error prints the mangled name as `HOST?`.

It had run fine every previous time because those runs were bash 5 with a
UTF-8 locale, which parses it correctly — a latent bug the whole time, not a
regression.

Rule: **no `$VAR` in a runtime string may be followed by a non-ASCII byte.**
Brace it (`${HOST}`) and keep echo/say strings ASCII; prose punctuation is
fine in comments, which never execute. Runtime strings in both scripts are
now ASCII (bar a few em-dashes with no adjacent variable). Reproduce any
suspicion with:

```sh
LC_ALL=C /bin/bash deploy/pi-deploy.sh --no-seed user@host
```

## Deploying to a Pi that is NOT this shack's (2026-08-27)

`deploy/pi-deploy.sh --no-seed <user@ip>`. Always, for any host that isn't
noderedpi4.

Default (seeded) mode ships `config/dxca.toml` and `data/{cty.xml,
lotw-users.txt,dxca.db}` alongside the binary, installed by install.sh
*only when absent*. On a box that already has its own files that guard makes
it a no-op — which is why it is safe for our own redeploys and dangerous
everywhere else, because a **fresh** host has nothing to guard against:

- `data/dxca.db` holds ClubLog app passwords, API keys and the Telegram bot
  token **in plain text** (by design, README §Secrets) plus account password
  hashes. Seeding it onto someone else's Pi hands all of that over.
- `config/dxca.toml` holds the cluster nodes with `login_call = "VU2CPL"`.
  Two hosts on the same node with the same callsign make DXSpider kick the
  duplicate, so both ends flap.

**Keep `--no-seed` on RE-deploys too**, not just the first install. It is
tempting to drop it once the remote box has its own config and database,
since install.sh then skips both — but that guard runs *after* the
transfer. `rsync` copies the whole staging directory to `~/dxca-deploy/` on
the remote host first, so without the flag this station's `dxca.db` ends up
sitting in someone else's home directory even though the installer
correctly declines to install it. The flag prevents the **copy**, which is
the part that matters.

What a re-deploy does and does not keep, on any host:

| | |
|---|---|
| `/opt/dxca/config/dxca.toml` | **kept** (written only if absent) |
| `/opt/dxca/data/*` — db, cty, LoTW | **kept** (same guard) |
| `/opt/dxca/dxca` | replaced — the point of the exercise |
| `/etc/systemd/system/dxca.service` | **overwritten unconditionally** from the template — any hand-editing of the unit is lost |

New schema and config keys need no manual step: the `meta` table is
`CREATE TABLE IF NOT EXISTS`, and every added key is `serde(default)`.

`--no-seed` ships only the binary, `deploy/dxca.service` and `install.sh`
(not even the vu2cpl-named macOS plist). The remote box self-bootstraps: the
first-run setup card creates *their* admin account, and cty.xml / the LoTW
list download on demand. Either way the script now prints a **manifest** of
what is about to leave the machine before it rsyncs — read it.

Remote-host preflight, because the binary is cross-compiled for aarch64 +
glibc ≥ 2.36:

```sh
ssh user@ip 'uname -m; ldd --version | head -1; . /etc/os-release && echo $PRETTY_NAME; sudo -n true && echo SUDO_NOPASSWD || echo SUDO_NEEDS_PASSWORD'
```

Wants `aarch64`, glibc 2.36+, Bookworm. 32-bit Pi OS or Bullseye needs a
different target triple. Over a VPN use the **IP** — `.local` mDNS names
generally don't resolve across the tunnel. The final install step now runs
under `ssh -t` so a host without NOPASSWD sudo can actually prompt.

## Deploy gotcha (fixed 2026-08-27)

`install.sh` ended with `systemctl enable --now dxca`. `--now` starts an
*inactive* unit and does nothing to an active one, so re-running the
installer over the live service **installed a new binary but kept the old
process running** — `sudo install` replaces the file, and the running
process holds the old inode. That is why the 2026-08-27 (late) installer
re-run read as "service undisturbed": it was, including the part that
should have been disturbed. Caught when deploying the UI restyle: the
process had started 03:17 while `/opt/dxca/dxca` was stamped 03:43.

Now `enable` + an unconditional `restart`. If you ever see the dashboard
not matching the code you just shipped, check
`systemctl show dxca -p MainPID,ActiveEnterTimestamp` against
`ls -l /opt/dxca/dxca` first.

## Web UI look

**2026-08-27 — the GUI was restyled to Meridian's design system.** Visual
only: same screens, same data, same information architecture, no API
change. Rust untouched, so only `just web` is the relevant gate (it
passes; `dist/` is gitignored and rebuilt by `just web` / `just run`).

What replaced what:

- `web-ui/src/app.css` is now a port of Meridian's stylesheet. The old
  base was hardcoded GitHub-dark (`--bg: #0d1117` …) — one appearance,
  with hexes re-typed per component. The new one derives every surface
  from the CSS **system colours** `Canvas` / `CanvasText` via
  `color-mix()`, so light and dark both come for free.
- New `web-ui/src/lib/theme.svelte.ts` + `ThemeSwitcher.svelte` (both
  ported): Auto / Light / Dark in the header, stored under
  `localStorage['dxca.theme']`, applied by pinning `color-scheme` on
  `<html>`. `index.html` re-reads that same key **before** first paint to
  avoid a flash — change one, change both.
- Shared vocabulary in app.css, used by every screen instead of per-view
  hexes: `.card`, `.pill`, `.status-dot` (`.on` / `.warn` / `.err` —
  replaces the old `.dot.green/.yellow/.red`), `.filter-chip`,
  `.settings-form`, `.hint`, `.actions`, `nav.tabs`, `.popup-menu`.
- **DXCA's own addition, with no Meridian counterpart:** the *alert
  ladder* (`--alert-dxcc` / `-slot` / `-band` / `-mode`, each with a
  matching `-bg` row wash). Same four hues 1.x used, re-expressed with
  `light-dark()` — the old `rgba()` tints only composited correctly over
  `#0d1117`. Level colour and row wash come from the one token, so the
  Alert cell and its tint cannot disagree.
- Header gained the version beside the wordmark (read off the bootstrap
  `/api/status` call — no extra request) and the theme toggle.

Licensing: the three ported files are **Apache-2.0**, like
`dxca-connect/src/dxcluster/`, not MIT. Each carries the note in its own
header; README's License section lists them. If you add to app.css,
mark it `DXCA:` at the site.

Verified in the embedded browser across all six screens (Spots, My
ClubLog, My Alerts, Users, System, and the setup/login card) in **both**
appearances, against a throwaway static server with stubbed `/api`
responses — deliberately not against the production Pi, since running a
second server locally would dial the real cluster nodes with the
production callsign.

Not done (out of the visual-only scope, would need new endpoints or
panels): the propagation / host / band-activity cards Meridian's own
dashboard carries, and any i18n.

Post-2.0 backlog (pick up whenever): per-user telnet feeds (Meridian
server lift), MQTT status/LWT on `shack/dxca/status` (broker is
localhost on the Pi!), durable spot history + search, possible Meridian
integration (plan §6), web editing for bind-level scalars, additional
decoder ports if the shack grows.

## Conventions (see ~/.claude/CLAUDE.md)

- **CDP** — Commit, Document, Push together on every substantive change.
- Repo is **private**; goes public only on explicit instruction.
- Credit VU3ESV (concept) and Meridian (telnet engine + the web GUI's
  design system) in any user-facing write-up — already in README.
