# DXCA

FT8/FT4 + DX-cluster spot aggregator with a multi-user web GUI. Rust
successor to [DXClusterAggregator for
macOS](https://github.com/vu2cpl/DXClusterAggregator-macOS), built to run
24/7 on a Raspberry Pi, and equally at home on macOS or Linux. See
[How to install](#how-to-install). (Windows: a prebuilt `.exe` with an
installer ships as a release asset as of 2026-08-28 — it builds, installs and
runs, but it is far less proven than the other three and stores secrets
unprotected. Read [Windows](#windows) before using it.)

DXCA ingests spots from WSJT-X/JTDX instances (binary UDP) and DX-cluster
telnet nodes, aggregates and dedupes them, and serves the result to logging
software over a built-in telnet cluster server and to UDP destinations
(including a verbatim passthrough that keeps loggers' click-to-fill
working). A web GUI with per-user accounts lets each operator carry their
own ClubLog log matrix, New-DXCC/Slot/Band/Mode highlighting, and Telegram
alerts over one shared spot stream.

Original concept and reference implementation by Vinod VU3ESV (FT8 Cluster
Aggregator); rewritten and extended. The DX-cluster telnet client engine
and the web GUI's design system are derived from the **Meridian**
project — joint work by Basil Thomas W6BT, Vinod VU3ESV, and Ram VU3RDD
(repo private).

## Status

**v2.20.4** (2026-09-01): **the Zone and Marathon rows in Alerts could not
be switched off.** Manoj: *"if dx marathon is not selected in alerts, still
notifications were coming ... have to disable in awards to stop the
notifications"*, and *"dx marathon if selected, selects zone also"*. Both
symptoms were one fault. The Alerts tab kept its own hand-typed table of
level → `notify_*` field, and it was never extended when WAZ and the
Marathon joined the ladder in 2.19.0: those three rows looked up **nothing**,
so all three bound to the same phantom slot — ticking DX Marathon appeared
to tick both Zone rows — and no save ever carried their fields, so the
server's default-on switched them back every time. The only thing that
actually stopped the pings was dropping the award on **Settings › My station
› Awards**, which is the classifier gate, not the Telegram one.

The field name is now **served with the level** (`/api/reference` carries
`notifyField`), so the ladder cannot grow a row the UI has no field for, and
a level that somehow arrives without one is dropped rather than drawn as a
checkbox that does nothing. Three tests pin it: every flaggable level names
a field that exists and that `wants_level` actually reads, no two levels
share a field, and every served level carries one.

**v2.20.3** (2026-09-01): **the Awards help tips said "confirmed" always
means LoTW, which is false for WAZ.** Manoj, reading the tab: *"why 2 zones
worked and confirmed"*. The pair is right — the tip explaining it was not.
Zones score off the ClubLog export alone: **worked** is any credited QSO
carrying a `CQZ`, **confirmed** is one whose record has a QSL flag
(`LOTW_QSL_RCVD`, `QSL_RCVD`, `EQSL_QSL_RCVD` or `APP_CLUBLOG_QSO_QSL`), and
`merge_lotw_confirmed` never touches zones. Only **IOTA, WAS and VUCC** need
the LoTW login. The Awards-tab tip and the Awards settings tip now split the
rule per award instead of claiming LoTW for all of them. Text only — no
count, score or setting changes.

**v2.20.2** (2026-09-01): **one type scale on the Awards tab, and WAZ shows
worked / confirmed everywhere.** Each section had grown its own size — counts
at 1.05rem, mono lists at the body size, table cells at whatever they
inherited — so the page read as several documents. Everything that is data
now takes `--fs-item`, the size the stylesheet already defines for it, and
every section heading is the plain `<h2>` the rest of the app uses inside a
card. The WAZ by-band and by-mode rows carry **both** numbers, because for
zones they genuinely differ.

**v2.20.1** (2026-09-01): **WAZ gains a mode split, and says whether it is
confirmed.** A third scope — **Per mode** — chases each zone in CW, Phone
and Digital, the WAZ counterpart of Triple Play. The card now shows zones
**worked and confirmed** rather than one number that looked like both, a
per-mode breakdown, and a *Still needed* worklist. Missing means **not
confirmed**, since an award is claimed on confirmations.

**v2.20.0** (2026-09-01): **more than one FlexRadio and more than one
ExpertSDR3 per account** — contributed by VU3ESV
([#2](https://github.com/vu2cpl/dxca/pull/2)). Both radio tabs now hold a
list with **Add radio** per row, and every alert goes to all of them.
Neither wire format changes, and neither client needed changing: both pools
were already keyed by `(host, port)`, so what was single was only ever the
*setting*. Addresses are deduped **after** the port is resolved, so
`192.168.1.60` with the port box blank and `192.168.1.60:40001` are one
radio rather than two marks per spot. **Existing single-radio setups are
adopted into a one-entry list on upgrade** — nothing to re-enter.

**v2.19.3** (2026-09-01): **"Still needed" covers mixed WAS too.** The
per-mode lists were there but the mixed one was stranded as a line in the
summary card, so the section that answers "which states do I still want"
did not actually answer it for basic WAS. Mixed now leads that list, and
the summary stops repeating it.

**v2.19.2** (2026-09-01): **the Awards tab reads as sections.** Its
subheadings were bold body text wedged against the numbers above them, so
three groups ran together as one block; they now wear the card heading
style used everywhere else — uppercase, letterspaced, muted, with a rule
and real space above. Counts are shown one way instead of two: label over
value, for modes and bands alike.

**v2.19.1** (2026-09-01): **the Awards tab is tidied.** "Triple Play still
needs" was laid out in a two-column grid that put two modes on one row and
orphaned the third; each mode now gets its own row, in the CW/Phone/Data
order the rest of the app uses. Triple Play's score was printed twice, in
the summary card and again below it — once is enough.

**v2.19.0** (2026-09-01): **WAZ, the DX Marathon, and a WAS you choose.**

**Which WAS you are chasing is now a setting**, not an assumption: *Mixed*,
*Triple Play* or *Per band*, under Settings › My station › Awards. It
changes what **alerts**, which is the point — chasing Triple Play means Ohio
on phone must wake you even though Ohio has been worked on CW for years.

**WAZ** — the forty CQ zones, mixed or per band. Zones come from your log's
own `CQZ`, which ClubLog *does* export, so this award needs no LoTW report.
**DX Marathon** — entities plus zones worked in the current calendar year,
one point each, reset every January; it earns its own alert because an
entity worked in 2019 is a fresh point today and every other level stays
silent on it.

Underneath both: **cty.xml's per-prefix CQ zones are now read** (they were
parsed and dropped), so VE7 resolves to zone 3 and VE3 to zone 4. cty.xml
has **no US call-area records at all** and answers 5 for the whole country,
so for US calls the zone is derived from the FCC state instead — without
which zones 3 and 4 could never be credited.

**v2.18.0** (2026-09-01): **WAS endorsements, Triple Play, and Awards as its
own tab.** WAS is now counted **per band and per mode class** as well as
overall, and **ARRL Triple Play** — all fifty states confirmed in CW, Phone
and Digital — gets a score *and a worklist*, because "39 of 50" cannot tell
you that Wyoming needs all three modes while Ohio needs only phone. DXCA
satisfies Triple Play's LoTW-only rule by construction: the QSL report is
the only thing that ever confirms a state. Awards moved out of a card under
My ClubLog into its **own Stats tab**.

**Also a real correctness fix:** LoTW's `STATE` is a worldwide subdivision
code, and several collide with US postal codes — China's Shandong is `SD`,
Brazil's Santa Catarina `SC`, Russia's Moscow oblast `MO`. The log side
never checked the entity, so a Shandong QSL credited South Dakota. Both the
ClubLog and LoTW merge paths now require a WAS-countable entity, the same
gate spots have had since 2.17.3. **A log refresh is needed** to rebuild the
mode axis and drop any false state.

**v2.17.7** (2026-09-01): **"All" gets its own line in every filter group** —
Sources, Alerts, Modes and Bands. It is a reset, not one of the choices, and
at the head of a ragged run of chips in a 12rem rail it read as one of them.

**v2.17.6** (2026-09-01): **the LoTW download is given time to finish, and a
failed one no longer erases what it cannot replace.** The full report that
2.17.5 started asking for takes longer than the 10-minute timeout allowed, so
it died mid-body and merged nothing — and because the matrix is rebuilt from
scratch, that published an empty WAS and IOTA. Three changes: the timeout is
**30 minutes**; a failed report now **carries the previous states and islands
forward** instead of replacing them with nothing; and *Refresh log now* says
it can take **10 minutes or more** rather than "a minute".

**v2.17.5** (2026-09-01): **the LoTW report is actually fetched in full.**
LoTW's QSL report is *incremental by default* — it answers "what is new
since you last asked" — and DXCA never pinned a start date. So the first
download brought your history and the next one brought a handful of
records, which then replaced it; since the log matrix is rebuilt from
scratch each refresh, every worked state and island vanished with it and
every US state started alerting as new. On this station the request that
should have returned **28,467 confirmations with 6,786 state fields** was
returning **one record**. It now asks from 1945.

Alongside it, a guard for the same class of failure: **an award axis with
no data claims nothing is new.** States and islands come only from that
optional report, so an empty map means "unknown", not "none worked" —
DXCA stays quiet instead of flagging every state on the band. Grids are
not guarded this way: they come from the ClubLog log itself, so empty
there genuinely means no 50 MHz+ grids worked.

**After upgrading, press Refresh log now** — the corrected report has to
be downloaded once before your states appear.

**v2.17.4** (2026-09-01): **the Server card and the file-only line tell the
truth.** The "Milestone" row said `2.1` next to a version that said 2.17 — it
was a hand-maintained string nothing updated, so it is gone. The file-only
settings line now lists **every** setting `config/dxca.toml` owns: the IOTA
and FCC refresh intervals added in 2.17.0 were missing from it, and
**whether the telnet server accepts logins** (`telnet_interactive`) was not
visible anywhere in the web UI at all.

**v2.17.3** (2026-09-01): **a US call operating abroad is no longer a New
State.** `DV2/K7AZQ` — an Arizona licensee in the Philippines — was credited
with Arizona, because the state lookup reused the callsign ladder that
answers *who a call belongs to* for a question about *where the operator
is*. Two independent guards now: the spot's DXCC entity has to be one WAS
counts, and the lookup itself takes nothing but a plain `/P`, `/M` or
`/QRP`. Also: the two settings pages are just **ClubLog** and **LoTW**.

**v2.17.2** (2026-09-01): **explanatory prose moved onto the `?` hovers.**
Settings pages carry their controls; the paragraphs explaining them are one
hover away. Conditional warnings — "the FCC table is not on this server yet",
"not set, so no confirmed totals" — stay on the page, because those are state
you need to see without going looking.

**v2.17.1** (2026-09-01): **ClubLog and LoTW are separate settings pages** —
two accounts at two organisations, and the LoTW login was easy to miss as a
footnote under the ClubLog form.

**v2.17.0** (2026-09-01): **awards beyond DXCC, and a settings page to pick
them.** **IOTA**, **WAS** and **VUCC** each get a New/`?` level pair, switched
on under **Settings › My station › Awards** — and an award you have not ticked
adds nothing anywhere, so the Spots and Alerts screens of anyone who ignores
this are unchanged. Islands come from cluster comments (validated against the
IOTA directory), states from the FCC licence database, grid squares from the
comment or an FT8 CQ (50 MHz and up, per the ARRL rule). Worked comes from
your ClubLog log; **confirmed** comes from a new optional LoTW QSL-report
download, because ClubLog's export carries no state or island detail. (WAZ,
added in 2.19.0, is the exception — zones score off the QSL flags the
ClubLog export does carry.)
See [IOTA, WAS and VUCC](#iota-was-and-vucc-optional-off-by-default).

Also in this release: a **confirmation-path gate** — the `?` levels can be
held to calls new to your log and/or on LoTW, so an unconfirmed entity stops
pinging for the station that never QSLed the first time
([Confirmation-path gate](#confirmation-path-gate-optional-off-by-default));
**Destinations gained a fourth tab, TCI**, putting the same alerts on an
**ExpertSDR3 panorama** (SunSDR and anything else ExpertSDR3 drives) over
WebSocket **40001**, independent of the FlexRadio tab and of Telegram
([ExpertSDR3 panorama (TCI)](#expertsdr3-panorama-tci)); and the embedded
ClubLog DX Dashboard now follows this app's light/dark appearance instead of
picking one by the clock.

**v2.16.0** (2026-08-30): the Settings rail is down to seven entries.
Everywhere a spot arrives from is one **Sources** page (UDP decoders, cluster
nodes), everywhere it leaves by is one **Destinations** page (UDP, MQTT,
FlexRadio) — each with tabs, and the FlexRadio panadapter settings moved there
from their own entry. Award totals agree with ClubLog's own, every uncredited
contact is named in the log at refresh time, and Telegram carries **health
alerts** and a green ❇️ on LoTW stations. **Existing installs need one ClubLog
refresh** before their totals move; stored matrices are not rebuilt on
upgrade. dxca is the
live shack aggregator with a real web GUI, and **runs on
Windows**, with a prebuilt `.exe` and installer as a release asset (read
[Windows](#windows) first; it is the least proven of the four platforms). Decoder UDP sources and
DX-cluster telnet nodes (Meridian-lifted client with the 1.x
honest-status graft) feed one pipeline into the telnet server, the RUMlog
passthrough, and a WebSocket-streamed spots dashboard — station card with
worked/confirmed DXCC, DXCC Challenge and slot totals, status pills,
live sortable table,
per-user alert row tints, LoTW markers. Spots are flagged across **fourteen
alert levels** — New and `?` (worked-but-unconfirmed) pairs for
DXCC/Band/Mode/Slot, plus opt-in pairs for the **IOTA, WAS and VUCC**
awards (island reference, US state, grid square — see the awards section) —
each independently switchable, and narrowable by level, mode class
(CW/Phone/Data) and band (160m–70cm) both on screen and, separately, for
Telegram — where the `?` levels can further be held to calls new to your
log and/or on LoTW, the stations that can still turn an entity green. The shell is **three tabs — Spots,
Alerts, Stats — and a gear** (2026-08-29): what you watch is a tab, everything
you set up lives behind the gear in **Settings**, grouped by whose it is (My
station / Server / Access) and searchable by topic. Both feeds are laid out on
one fixed measured grid, so a column lands on the same x in every row while the
stream runs, and each screen's narrowing sits in a collapsible left rail that
reports how much it is holding back even when folded. The GUI wears Meridian's
design system (2026-08-27): one card/pill/table vocabulary across every screen,
contextual `?` help on hover, and
**light and dark appearances** that follow the OS unless the header's
toggle pins one. SQLite-backed accounts (argon2 + session
cookies, first-run setup card) each carry their own ClubLog matrix,
re-downloaded on a per-account schedule (daily by default) so a QSO worked
today stops showing as new tomorrow, with ClubLog's own DX Dashboard embedded
under My ClubLog — re-tinted to follow this app's appearance, because left
alone that page picks light or dark by the clock — and a **band × mode
grid** of entities worked and confirmed —
a row per mode class against every band, with a Total column and a Mixed row,
the RUMlog layout; the shared LoTW users list refreshes
server-wide, weekly by default. Every spot classifies per user with
Telegram alerts and per-callsign cooldown; a send that fails in transit
(handshake or response timeout) is retried once, and every send — including
failures, with Telegram's own error text — lands in the Alerts history.
Proven end-to-end in tests against fake ClubLog/Telegram
servers and by the live validations along the way (RUMlog click-to-fill,
honest-yellow flaky node, exact matrix parity with the 1.x app's own
artifacts). Sources, cluster nodes, and broadcast destinations are
edited under Settings › Server and hot-apply — listeners rebind, nodes redial,
destinations re-point, and `config/dxca.toml` is rewritten so restarts
agree. Ships as a launchd agent (macOS) or systemd service (Pi);
**in production on the shack's Raspberry Pi since 2026-08-27**, on a Windows
machine on the same LAN, and on two further, unrelated stations' Pis.

New since 2.2: every spot names **both** the feed that carried it and the
station that actually spotted it ([Who spotted it](#who-spotted-it)),
carried through to Telegram and the alert history; **skimmer spots are
marked and filterable**, on screen and for Telegram separately; a **search
box** over callsign and spotter; award totals that count **current DXCC
entities by default** ([Deleted entities](#deleted-entities)); and an
optional **interactive telnet** mode where an operator logs in and passes
read-only cluster commands through to one node
([Telnet login](#telnet-login-optional-off-by-default)) — off by default,
and invisible to loggers either way. Telegram sends now retry once when
they fail in transit. From 2.4.0 the database migrates itself on open, so
an upgrade needs no manual step.

The full design and milestone plan: [docs/PLAN.md](docs/PLAN.md). The 1.x macOS
app (final release v1.8.4) is the retained fallback.

Secrets note (plan §5): per-user ClubLog app passwords and Telegram
tokens live in `data/dxca.db` in plain text, file mode 0600, service user
only — encryption-at-rest on the same host would add ceremony, not
security. Keep the data directory out of backups you share.

## How to install

Written for someone who has never built a Rust program. You end up with a
service that starts on boot and a web GUI on port **7580** that you open
from any machine on the LAN.

Whatever the platform, the shape is the same: install two toolchains (Rust
and Node), clone this repo, run `./install.sh`, open the URL. The installer
auto-detects your platform, asks you to confirm, refuses to continue if
anything it needs is missing or too old, and — since it finishes by fetching
its own web page — tells you whether the install actually worked rather than
just that it finished.

### What you need first

| | Why | Minimum |
|---|---|---|
| Rust (via **rustup**) | builds the server | **1.88** |
| Node + **pnpm** | builds the dashboard | Node **18, 20, or 22+** |
| git | to clone this repo | any |

Node is not a plain minimum: vite and the Svelte plugin both declare
`^18 || ^20 || >=22`, so the odd-numbered non-LTS releases **19 and 21 are
rejected** even though they are newer than 18. Install an LTS — 22 is a safe
choice. `install.sh` checks this before it builds anything.

**Install Rust with rustup, not your distro's package manager.** Debian
Trixie's `apt install cargo` gives 1.85, which is below the floor this
workspace needs, and a distro rustc ignores `rust-toolchain.toml` so it will
never fix itself. See [Build](#build) for why the floor is 1.88.

### Raspberry Pi

Needs a **64-bit** Raspberry Pi OS (Bookworm or newer). Check with `uname
-m`: `aarch64` is good, `armv7l` means you are on the 32-bit image and this
will not run. A Pi 4 or 5 is comfortable; on a Pi 3B the build is slow and
may run out of memory — see [Low-memory Pis](#low-memory-pis) below.

**1. Update the system and install git.**

```sh
sudo apt update && sudo apt install -y git curl build-essential
```

**2. Install Rust.** Accept the default option when it asks.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version          # expect 1.88 or newer
```

**3. Install Node and pnpm.** If `node --version` already prints 20 or
higher, skip straight to the `pnpm` line — and **do not** `apt install npm`
alongside an existing Node, because a NodeSource `nodejs` package provides
its own npm and conflicts with Debian's.

```sh
node --version || sudo apt install -y nodejs
sudo npm install -g pnpm
```

**4. Get DXCA and install it.**

```sh
git clone https://github.com/vu2cpl/dxca.git
cd dxca
./install.sh
```

It asks `Auto-detected platform: pi. Is this correct? [Y/n]` — press Enter.
Then it builds (ten to thirty minutes on a Pi; the Rust compile is the slow
part), installs to `/opt/dxca`, starts a systemd service called `dxca`, and
checks the result. Success ends with `OK: http://127.0.0.1:7580/ is serving
the dashboard.` and the LAN address to use.

**5. Open the web GUI** at the LAN address it printed, e.g.
`http://192.168.1.50:7580/`.

Useful afterwards:

```sh
systemctl status dxca              # is it running
journalctl -u dxca -n 50 --no-pager   # what it said
```

### macOS

**1. Install [Homebrew](https://brew.sh)** if you do not have it, then the
toolchains:

```sh
brew install rustup node pnpm git
rustup-init -y
source "$HOME/.cargo/env"
```

**2. Install DXCA.**

```sh
git clone https://github.com/vu2cpl/dxca.git
cd dxca
./install.sh
```

Confirm `macos` when asked. It builds, installs a launchd agent called
`com.vu2cpl.dxca` that survives reboots, and verifies the result. Unlike the
Pi, macOS runs the binary **from the clone you just made** — so do not move
or delete that directory.

**3. Open** `http://localhost:7580/`.

The log is at `~/Library/Logs/dxca.log`. To stop it:

```sh
launchctl bootout "gui/$(id -u)" ~/Library/LaunchAgents/com.vu2cpl.dxca.plist
```

### Other Linux (x86-64 or ARM)

Identical to the Pi, with your own package manager for step 1 — the
installer detects `linux` instead of `pi` and installs the same systemd
service to `/opt/dxca`. Everything else is the same.

### Windows

**Works, but is the least proven of the four platforms.** First built and run
on 2026-08-28. Read this whole section before installing.

Download `dxca-<version>-windows-x64.zip` from the releases page, unzip it
anywhere — Downloads is fine — then right-click **`install-dxca.cmd`** and
choose *Run as administrator*. It installs into **`C:\DXCA`** regardless of
where you unzipped, and runs `dxca.exe` from there as a LOCAL SYSTEM
scheduled task with a boot trigger, optionally opening the firewall. The
unzipped folder is only the delivery package and can be deleted afterwards.
`uninstall-dxca.cmd` reverses the task and firewall rules and leaves
`C:\DXCA\config\` and `C:\DXCA\data\` alone.

Strictly the location is `%SystemDrive%\DXCA` — the drive Windows itself
booted from, which is `C:` on any ordinary machine and `D:\DXCA` on one whose
Windows lives there. It is found automatically, by the installer and by
`deploy/win-deploy.sh` alike. What you cannot do is *choose* a drive: there is
no prompt for it, because one fixed location is precisely what stops an
upgrade having to ask.

The fixed location is what makes upgrades uneventful — see
[Updating](#updating). It is also locked to administrators when the
installer creates it: a folder at the root of `C:` otherwise inherits the
drive root's ACL, which would let any standard user replace an executable
that Windows then runs as SYSTEM.

`dxca.exe` is one self-contained file — the dashboard is embedded and every
DLL it imports is a Windows system library or Universal CRT. No Rust, no
Node, no Visual C++ redistributable, and a config file is optional (a missing
one yields working defaults).

What you are getting, stated plainly:

- **Secrets are not protected.** ClubLog app passwords and Telegram tokens
  live in plain text in `data\dxca.db`, secured on Unix by mode `0600` — a
  `#[cfg(unix)]` path that Windows skips entirely. The installer's ACL on
  `C:\DXCA` keeps non-administrators out of the file, which is not the same
  as protecting its contents: anyone who can elevate reads it. Use a ClubLog
  *app password*, never your main one. This is the one gap that keeps Windows
  from being "supported" rather than "works".
- **Receiving spots is untested on Windows.** No WSJT-X, JTDX or cluster node
  has fed the Windows build. Serving, storage, the web GUI, the telnet server
  and the installer are all verified; the ingest path is not.
- **The shipped binary is a GNU/mingw cross-build** made on macOS with
  `cargo-zigbuild` (`just win`), not what MSVC would produce. A native MSVC
  build needs the **Visual Studio Build Tools with the C++ workload**, because
  bundled SQLite and `ring` both compile C. That route has never been tried.
- **It is unsigned**, so SmartScreen will warn and some antivirus flags
  unsigned Rust binaries. There is no code-signing certificate for this
  project.
- **Graceful shutdown is unexercised** — the Windows path handles Ctrl-C only,
  and the service is stopped by termination.
- Tested on exactly one machine: Windows 10 22H2, build 19045, AMD64.

Two Windows-specific traps the installer handles for you, worth knowing if
you set it up by hand instead:

- **Create the admin account before opening the firewall.** The first-run
  setup card is unauthenticated, so on an open port whoever loads it first
  claims admin. The installer runs on loopback and waits for the account.
- **Firewall rules are scoped to the Private profile.** Windows classifies an
  unrecognised network as *Public*, and re-classifies on its own when an
  adapter re-identifies. When that happens the rules are present, enabled and
  completely inert — the server listens and nothing can reach it. The
  installer detects this and says so rather than reporting success.

If you want certainty rather than novelty, run DXCA on a Pi or a Linux box
and open the web GUI in the Windows machine's browser. That configuration is
the one in daily production use.

### First run

The first page is a setup card, because no account exists yet and there are
no default credentials, ever. Create the admin account — that callsign and
password are yours alone; nothing is pre-seeded.

You do **not** need a ClubLog API key. A released dxca carries one, so
cty.xml — the DXCC prefix database every spot is resolved against — downloads
on its own. (The field is still there, under **Settings › Server › Reference
data → Advanced**, for an admin who would rather spend a key of their own, or
if you built from source without one. See
[Building without a ClubLog key](#building-without-a-clublog-key).)

Then, in order of what actually matters:

1. **Settings › Server › Cluster nodes.** Add the DX-cluster node(s) you want
   ingested, with your callsign as the login.
2. **Settings › My station › ClubLog account.** Your ClubLog credentials, so your own log loads and
   New-DXCC highlighting means something for your station.
3. **Point your decoders at it.** WSJT-X/JTDX/MSHV UDP to ports 2333 (MSHV),
   2334 (JTDX), 2335 (WSJT-X). Point your logger's telnet cluster at port
   **7575** on this machine.

   That is the whole decoder setup — **no secondary / 2nd / Simplified UDP
   broadcast is needed in any of the three.** Logged QSOs travel on the same
   socket as the decodes (WSJT-X type-5) and reach your logger through the
   passthrough destination, so one feed per decoder covers spots, click-to-fill
   and QSO logging alike. The one exception is **MSHV**, whose broadcast gates
   each message type separately: tick **Enable Logged QSO** there, or its QSOs
   never leave. JTDX and WSJT-X need nothing beyond the port.

### Updating

**Same script, every platform.** There is no separate updater — `install.sh`
is idempotent, and re-running it *is* the update:

```sh
cd dxca
git pull
./install.sh
```

macOS reloads the launchd agent; Pi and Linux reinstall to `/opt/dxca` and
restart the systemd service. Either way it finishes by fetching the page and
telling you whether the new version is actually serving, so a silent no-op
update is not a thing that can happen.

**`install.sh` does not pull, and says so if you forgot.** It builds the
working tree exactly as it stands — an installer that pulled would be taking
a decision about your code, fighting local edits and tripping over a
detached HEAD. Instead it checks whether your checkout is behind its
upstream and tells you before spending ten minutes rebuilding the old
version:

```
NOTE: this checkout is 7 commit(s) behind origin/main.
install.sh builds the working tree as it stands — it does not pull.
To install the latest instead, stop now and run:
  git pull && ./install.sh
```

It is a note, never a stop — installing an older checkout is a legitimate
thing to do. Nothing is printed when you are current, when there is no
upstream, or when there is no repo at all (a `pi-deploy.sh` bundle).

**Nothing you configured is touched.** Accounts, ClubLog credentials, alert
preferences, the worked matrix, your cluster nodes and ports all live in
`config/dxca.toml` and `data/` on the running host, and the installer writes
those **only when absent**. There is no migration step either: the database
schema is applied as `CREATE TABLE IF NOT EXISTS`, so an older database just
keeps working. `git pull` cannot conflict with your settings, because both
paths are gitignored — nothing you edit is tracked.

Confirm what is actually running, rather than what you think you installed:

```sh
curl -s http://localhost:7580/api/status | grep -o '"version":"[^"]*"'
```

**Updating a Pi from your Mac instead.** If the Pi is slow, or you would
rather not build on it at all, cross-compile here and ship the finished
binary — the Pi needs no Rust or Node toolchain for this route:

```sh
deploy/pi-deploy.sh --no-seed user@192.168.1.50
```

Use the **IP** over a VPN; mDNS `.local` names usually do not resolve across
a tunnel. Keep `--no-seed` for any Pi that is not your own — and on re-runs,
not just first installs. Without it, `rsync` copies your `data/dxca.db`
(ClubLog app passwords, Telegram token, account hashes in plain text) and
your `config/dxca.toml` (with *your* callsign as the cluster login, which
makes both stations fight over the same node session) into that host's home
directory. The installer correctly declines to *install* them, but the guard
runs after the transfer — the flag is what prevents the copy.

**Windows** updates by unzipping the new release anywhere and running
`install-dxca.cmd` as administrator. Because DXCA lives in `C:\DXCA` rather
than in the folder you unzipped, an upgrade only stops the service, replaces
the binary there and re-registers the task — `config\` and `data\` are
already in place and are never touched, so accounts and settings carry over
with nothing to answer.

**Updating Windows from your Mac instead.** If the Windows box runs OpenSSH
Server and you can reach it with a key, `deploy/win-deploy.sh` does the same
job as `pi-deploy.sh`: cross-compile here, upload the exe, stop the task,
swap the binary, restart and verify — `config\` and `data\` untouched, and
an automatic rollback to the previous binary if the dashboard does not come
back.

```sh
deploy/win-deploy.sh user@192.168.1.170
```

It **updates**, it does not install: it registers nothing, so the first
install on any machine is still `install-dxca.cmd` from the release zip.
The SSH user must be in `BUILTIN\Administrators` with the group enabled, or
the session cannot control the task. Leave the SSH `DefaultShell` as
`cmd.exe` too — every remote command in the script is cmd syntax, which
PowerShell does not understand. Both, and every other precondition, are
checked *before* anything stops.

Installs made before v2.10.0 ran from the unzipped folder, so each release
landed in an empty new one and had to be set up from scratch. The first run
of a 2.10.0-or-later installer on such a machine finds no install in
`C:\DXCA`, offers to import the old one (it looks in the folder you
unzipped, in any `dxca-*-windows-x64` folder beside it, and in the existing
scheduled task's path), and copies the database, config, `cty.xml` and LoTW
list across. That question is asked once.

### If something goes wrong

The installer stops with an explanation rather than half-finishing, so read
what it printed. The common ones:

| It says | Meaning |
|---|---|
| `rustc 1.85.0 is too old` | distro Rust; install rustup as above |
| `pnpm not found` | step 3 was skipped — it prints the exact command for your machine |
| `Node 21.x cannot build the dashboard` | a non-LTS Node; install 22 |
| `never answered` | it built and started but is not serving; the message names the log command |
| `serving the PLACEHOLDER page` | the running binary was built without the dashboard |
| `nodejs : Conflicts: npm` | you asked apt for `npm` next to a NodeSource Node — drop `npm` |

Re-running `./install.sh` is always safe. In a source tree it rebuilds from
scratch, so it is the correct fix after installing a missing toolchain — it
will not reuse the old binary.

#### Low-memory Pis

On a Pi 3B (1 GB) the release build can be killed while linking. Either give
it swap, or build the binary on a faster machine and copy it over with
`deploy/pi-deploy.sh --no-seed user@pi` — that cross-compiles on your Mac
and ships a finished binary, so the Pi needs no Rust toolchain at all. Keep
`--no-seed` for any Pi that is not your own: without it the deploy also
copies your database and config, which carry your passwords and your
station's cluster login.

## Layout

| Path | What |
|---|---|
| `crates/dxca-core` | Pure logic: spot model; WSJT-X codec, parsers, matrix, classifier land in M1. No I/O. |
| `crates/dxca-connect` | I/O engines (M2–M4): DX-cluster telnet, WSJT-X UDP, broadcaster, ClubLog/LoTW/Telegram. |
| `crates/dxca-server` | Composition root: config, axum web API, embedded UI; auth + SQLite in M4. Binary is `dxca`. |
| `web-ui/` | Svelte 5 + Vite + TypeScript (pnpm). Built `dist/` is embedded into the binary. `src/app.css` is the design system — every colour is derived from CSS system colours, so the UI follows the OS light/dark and the header's toggle can pin either. |
| `config/dxca.example.toml` | Global config template — copy to `config/dxca.toml`. |

## Build

Needs **Rust ≥ 1.88** (stable, via rustup) and — only for the web UI — pnpm
with a Node in `^18 || ^20 || >=22` (vite's and the Svelte plugin's declared
engines; 19 and 21 are excluded). Plain `cargo build` never requires Node
at all (a stub page is embedded when `web-ui/dist` hasn't been built).

That 1.88 floor comes from the committed `Cargo.lock` (`ureq` → `url` →
`idna` → `icu_*`), not from dxca's own code, so it cannot be lowered by
editing this workspace. Distro packages often sit below it — Debian Trixie
ships 1.85.0 — and a distro rustc ignores `rust-toolchain.toml`, so install
rustup rather than `apt install cargo`.

It is declared as `rust-version` in `[workspace.package]`, so cargo refuses
an old toolchain immediately rather than after resolving the dependency
graph. `install.sh` checks it too, before the web build and before any
sudo, and tells you which of the two situations you are in.

```sh
cargo build --workspace          # all crates
cargo test --workspace
pnpm -C web-ui install && pnpm -C web-ui build   # real web UI into dist/
cargo run -p dxca-server         # http://localhost:7580
```

A [Justfile](Justfile) wraps the common flows (`just gate`, `just run`,
`just dist`) but is never required.

### Building without a ClubLog key

Released builds carry a ClubLog developer API key so a fresh install can
download cty.xml with nothing to register. It is **not in this repository**
and never will be: ClubLog's
[API Keys](https://clublog.freshdesk.com/support/solutions/articles/54910-api-keys)
article says keys found published on the web or in a Git repository are
deleted without notice, and this repo is public.

So a build from source has no key unless you supply one, and behaves the way
dxca always did — an admin sets a key in **Settings › Server › Reference data
→ Advanced** and cty.xml downloads from there. Everything else works
regardless; only the country file needs the key.

To bake your own in, put it where `crates/dxca-server/build.rs` looks:

```sh
echo <your-40-hex-key> > .clublog-api-key   # git-ignored
# or:  export DXCA_CLUBLOG_API_KEY=<your-40-hex-key>
cargo build --release -p dxca-server
```

Every release path picks it up with no extra step — `just dist`, `just win`,
`deploy/pi-deploy.sh`, `install.sh` — because they all run cargo in the same
shell. The build fails loudly on a key that is not 40 hex characters rather
than shipping a binary that 403s in the field, and the key is XOR'd on the way
in so it does not show up in `strings`. That last part is not encryption and
is not meant to be: anyone with the binary can recover it, which is why the
answer to a leak is to rotate the key, not to hide it better.

If ClubLog ever answers **403** for the key in use, dxca stops sending it —
automatic cty refreshes go quiet and Reference data shows "API key rejected".
That is deliberate: ClubLog ask that a rejected credential not be retried, and
they firewall hosts that keep trying, which would cut off every account on the
server. Enter a different key and refreshes resume on their own; there is no
reset to press. The same latch applies per account to the ClubLog log login.

[Request a key](https://clublog.org/requestapikey.php) if you want one of your
own. ClubLog issue them per *application*, so one key covers a whole server —
it is not a per-operator credential, and it is not what downloads anybody's
log (that is each user's own email and app password).

### Install as a service

Step-by-step instructions are in [How to install](#how-to-install); this is
the mechanism behind them.

`./install.sh [macos|pi|linux] [--stub-ui]` auto-detects the platform,
confirms, and never fails silently:

- **macOS**: builds and installs a launchd agent (`com.vu2cpl.dxca`,
  survives reboots, log in `~/Library/Logs/dxca.log`), running the binary
  from the clone.
- **Pi/Linux**: installs binary + config + data seeds to `/opt/dxca` and a
  systemd service (`systemctl status dxca`) running as the invoking user. A
  fresh install self-bootstraps: the first-run web card creates the admin
  account, and cty.xml / the LoTW list download on demand — no seed files and,
  with a released build, no ClubLog API key to obtain first.

Three things it guarantees, each of which was once a bug:

1. **Toolchain checked up front.** A missing or too-old Rust stops it before
   the build rather than minutes into dependency resolution.
2. **The dashboard is really built.** It is embedded at compile time, so
   installing it means building `web-ui/dist` *then* the binary, in that
   order. In a source tree the binary is **always rebuilt** rather than
   reusing an existing `target/release/dxca`, otherwise a re-run after
   installing pnpm would keep the old placeholder-page binary. A missing
   pnpm is a hard stop; `--stub-ui` opts into the placeholder deliberately
   (the API and telnet server work either way).
3. **The result is verified.** It waits for the service to answer on the
   configured port and checks the page is the dashboard and not the
   placeholder, exiting non-zero if not — so "installed" means serving,
   not merely "the script reached the end".

To cross-compile on the Mac and ship to a Pi in one step
(needs cargo-zigbuild + the `aarch64-unknown-linux-gnu` target):

```sh
deploy/pi-deploy.sh vu2cpl@noderedpi4.local
```

The aarch64 binary targets glibc ≥ 2.36 (Raspberry Pi OS Bookworm+,
64-bit). Existing config and data on the Pi are never clobbered —
`install.sh` seeds them only when absent.

## Configuration

Global (admin) settings live in `config/dxca.toml` — see the committed
[example](config/dxca.example.toml) — and, once the server runs, in the
web UI's Settings › Server pages (hot-applies and rewrites the file). Defaults keep
the shack wiring: web GUI **7580**, telnet cluster server **7575**,
decoder sources MSHV **2333** / JTDX **2334** / WSJTX **2335**,
passthrough → RUMlog **2237**.
Per-user settings (ClubLog credentials, alert preferences, Telegram) are
managed in the web GUI per account, not in the file.

### Who spotted it

Two different questions, two columns. **Source** is the feed that carried the
spot — a decoder ("MSHV") or the cluster node that relayed it ("N2WQ-2",
"HamAlert"). **Spotter** is the station whose receiver actually heard the DX,
taken from the `DX de …` line with any skimmer `-#` suffix stripped. On a
relaying node those are rarely the same, and only the second one tells you
whether a spot came from a skimmer wall or a human two hops away. Locally
decoded spots show no spotter — the source already names the receiver.

Telegram alerts carry both on their own line, plus the spot's own time in
UTC:

```
Spotter: VU2XYZ   Node: N2WQ-2
1428Z
```

**A LoTW station is marked with a green ❇️ after its callsign**, the same fact
the Spots table shows as a green dot:

```
🔴 NEW DXCC: 3Y0J❇️
Bouvet  14.074 MHz  20M  FT8  -10 dB
```

Telegram's HTML supports `<b>`, `<i>` and `<a>` but **no colour attribute**,
so the only green it will render is an emoji that is green in the font itself.
`❇️` is the one of those shaped like an asterisk rather than a dot, a tick or a
heart — a footnote mark, which is what this is. It is emoji-sized rather than
punctuation-sized; that is the trade for the colour.

The mark belongs to the callsign, so it appears at every alert level, and it
means what it does on screen — the station uploads to Logbook of the World, so
the QSO stands a good chance of confirming without a card chase.

Labelled rather than joined into a sentence, because on a phone those two
labels are what you scan for. Locally decoded spots show only `Node:` —
there is no spotting station to name. The time is the spot's, not the
delivery time, so a retried or queued alert still says when the station was
heard.

The search box above the table filters on **either** — type a DX callsign to
follow one station, or a spotter to see everything one skimmer is hearing.

A skimmer spot is marked with a small `#` after the spotter, and **Manual
only** hides them. The marker matters because the parser strips the `-#`
off the callsign to keep it readable: without it, `W3LPL` the operator and
`W3LPL-#` the skimmer would be indistinguishable on screen, though they are
not the same kind of spot at all.

**Telegram has its own** *only ping for spots a human made* switch in My
Alerts, independent of the screen filter — so you can watch everything and
still be interrupted only by people. On this station skimmers are roughly
three quarters of the feed.

Note the Spots filter is a **display** filter, not a node-side one. DXCA holds a single
cluster session per node, shared by every account and by the spot pipeline,
so a `reject/rbn` sent to the node would narrow everyone's feed and persist
on the node account — which is why the telnet passthrough refuses those
commands and this exists instead.

**Alerts** records the spotter alongside the source for every alert, so
the history answers the same question after the fact. Alerts sent before
this shipped show `—`: the column was added to existing databases on
upgrade, and there was nothing to back-fill it with.

### Confirmation-path gate (optional, off by default)

The four `?` levels exist to hunt confirmations, and a confirmation needs
the right station: some operators simply never QSL, and re-working one
cannot turn an entity green. Two ticks on **Alerts**, under *For the ?
levels*, narrow those pings to stations that still can:

- **The call is new to my log** — hold `?` pings for calls already in your
  log. A call you worked that never confirmed is a demonstrated non-QSLer.
- **The call uses LoTW** — hold `?` pings for calls not on the LoTW users
  list, the same server-wide list the green markers read. A LoTW user is
  the fast path to a confirmation.

With both on, only a call you have never worked that uploads to LoTW will
ping — a station that can be worked *and* will confirm. Both narrow the
`?` half of the ladder only: the **New** levels always follow their own
ticks, because an ATNO is worth working whatever the QSL prospects. Like
every other narrowing here it gates Telegram and the panadapters, never
the screen — the spot still shows with its tint; you are just not woken
for it. Design in [`docs/AWARDS.md`](docs/AWARDS.md).

### IOTA, WAS and VUCC (optional, off by default)

Three awards beyond DXCC, each with its own New/`?` level pair
(`docs/AWARDS.md` phases 2–4). **Settings › My station › Awards** is where
you pick your awards: the DXCC ladder at the top (it moved here from
the ClubLog page), then one tick per award — **IOTA**, **WAS**, **VUCC**
— that reveals its New/`?` pair. **An award left unticked does not exist
anywhere in the app** — no chips on Spots, no rows in the Alerts ladder, nothing on
Stats — so an operator who never opts in keeps exactly the classic
eight-level screen. Chased levels behave like any others: row tints and
chips on Spots, "Ping me for" narrowing on Alerts, colours on the
panadapters, and the alert names the catch — *New Grid MK83*, *? State
OH*, *New IOTA AS-153*.

What each level runs on, spot side:

- **VUCC (grid)** — the DX station's locator, from the cluster comment or
  an FT8 CQ. Counts 4-character squares **per band, 50 MHz and up only**
  (6m, 2m, 1.25m, 70cm, 33cm, 23cm — the ARRL rule; `RR73` is always a
  sign-off, never a square).
- **WAS (state)** — which US state the call is licensed in, looked up in
  the FCC amateur database. An admin downloads that once under **Settings
  › Server › Reference data** (~200 MB, distilled to ~8 MB on the spot);
  until then the State levels stay quiet. **A station operating away from
  its licence earns no state**: `DV2/K7AZQ` is an Arizona licensee
  transmitting from the Philippines, and a suffix override or a call-area
  digit (`W1AW/7`) is refused for the same reason. The caveat that remains
  is the one no logger escapes — the FCC knows the *licence* address, so a
  plain `W6` call held by someone who has moved to Ohio still reads as
  California. DC counts as Maryland, per the WAS rules.
- **IOTA (island)** — the reference a cluster spot's comment announces
  (`AS-153`), validated against the IOTA directory, which downloads from
  iota-world.org on the same Reference data page (their terms are personal
  non-commercial use, so it is fetched at runtime, never bundled). FT8
  decodes never name an island, so IOTA levels ride cluster spots only.

Log side, **worked** comes from your ClubLog download: grids ride its
`GRIDSQUARE` field; states and islands appear only if your uploads carried
them. **Confirmed** is where LoTW comes in: ClubLog's export carries no
state, island or QSL detail at all, so **Settings › My station › LoTW
account** takes an optional **LoTW website login** — your LoTW QSL report
is then merged at every log refresh (downloaded weekly, cached in
`data/`), and it is the authoritative source for confirmed states, grids
and islands.
Totals live on **Stats › My ClubLog** — an Awards card showing only what
you chase: IOTA and WAS counts with the missing-states chase list, and
grids per VUCC band.

### Stats

A **Stats** tab answers what the feed is actually made of: the total spots
held in memory, then breakdowns by band, by mode and by source. Counted
server-side across the **whole** spot ring rather than the 500 the Spots
screen holds — that is about five minutes on a busy feed, which would
answer a much smaller question than the one being asked.

Bands stay in band order so the chart reads as a band plan; modes and
sources sort by count. Percentages are of the ring, so each breakdown sums
to 100.

Deliberately **bars, not pie charts**: the job is comparing magnitudes
across up to fifteen categories with names like `UberSDR CWskim`, and a
fifteen-slice pie can neither be compared by eye nor hold its own labels.

### Band mask (optional, off by default)

Bands rotate through the day. At local midday 160m is dead for anything but
ground wave, and a New-Band flag on a 160m spot is an interruption you can
do nothing about; at 0200 local it is the most valuable line on the screen.

Set your **Locator** under *Settings › My station › Locator & grey line* — a 4- or 6-character Maidenhead square
— and a **Band mask** tickbox appears with the other narrowings on Spots.
With it on, spots whose band is not plausibly workable from your QTH at this
moment are **dimmed, never hidden**, and hovering a dimmed row restores it in
full. A `N dimmed` badge sits beside the spot count, because a filter that
silently changes the screen is indistinguishable from a feed going quiet.

Pick **dim** or **hide** beside the tickbox. Dim is the default and cannot
cost you a contact; hide is cleaner on a busy feed and is the deliberate
choice. Either way the count stays, and **New DXCC is never masked** whatever
the sun is doing.

It works from **where the sun actually is at your station**, not clock hours:
sunset moves by an hour across the year in Bengaluru and by six in northern
Europe, and above the Arctic circle clock rules stop meaning anything. Your
day is divided into four phases — **Dawn, Day, Dusk, Night** — and the badge
beside the tickbox says which one you are in, with sunrise and sunset in its
tooltip.

**Dawn and Dusk are the grey line**, the window either side of the terminator
when the low bands come alive and the high bands are still open. How long
that window lasts is **yours to set**, in minutes, beside the locator —
45 by default, because how long it stays useful varies with the band, the
season and the path. It is the one number in this feature you are expected to
nudge.

**Telegram narrows separately.** On *Alerts*, "only ping for bands that are
plausibly open right now" applies the same mask to your alerts while the
screen keeps showing everything. It fails open harder than the screen does: a
New DXCC always pings, and so does a band the model says nothing about,
because a held alert is a spot you never learn about at all.

Stated plainly, because a mask pretending to be a propagation predictor
would be worse than none: it models **only your end** of the path, uses no
solar flux, K index or MUF, and knows nothing about your antennas. It is a
coarse plausibility filter. Leave the locator blank and none of it exists.
Full reasoning in [`docs/PHASE-ROTATION-MASK.md`](docs/PHASE-ROTATION-MASK.md).

### Deleted entities

DXCC has a **deleted** list — Abu Ail, Blenheim Reef, British North Borneo
and 59 others. Those QSOs are real contacts and stay in your log, but they
score nothing toward current DXCC or the Challenge.

**Award totals count current entities by default**, which is what the ARRL
publishes and therefore what you are comparing against. An *include deleted
entities* tickbox on the **Spots** station line and the **Stats › My ClubLog**
statistics adds them back when you want the historical figure. One shared
preference, so the two screens can never disagree, remembered per browser;
both sets of totals are sent together, so toggling is instant.

The tickbox only appears once cty.xml is loaded — without it there is no way
to know which entities are deleted, so DXCA offers no choice it cannot
honour.

### Operations ClubLog does not credit

A prefix match is not a DXCC credit. cty.xml carries two further lists that
say so, and DXCA honours both — which is what makes its award totals agree
with ClubLog's own.

**Invalid operations** — around 2,800 callsigns, most bounded by a date
window, whose QSOs do not count: pirates, unlicensed activity, DXpeditions
whose paperwork was rejected after the fact. The windows are minute-accurate,
not day-wide. Some are only an hour or two long — the T6AA entry covers 19:00
to 20:15 on one evening in 2019 — so a contact earlier the same day is
perfectly valid and is counted.

**Whitelisted entities** — 59 of the rarest, from Bouvet and Navassa to
Kermadec, Mount Athos and North Korea. For these, ClubLog credits *only* the
callsigns it has explicitly listed; anything else that merely matches the
prefix earns nothing. Ten of them are whitelisted from a date rather than
always: Turkmenistan from 2007, Iran from 2019, the Pacific islands from the
1978 rule change, and ordinary calls before that still count.

The whitelist is what produced VU24DX's **314 DXCC worked against ClubLog's
313**. One `ZL8AC` QSO on 40m: `ZL8` resolves to Kermadec, but ZL8AC appears
nowhere in cty.xml, so ClubLog gave no credit and DXCA — which read the prefix
and stopped there — did.

Three details worth knowing:

- **The QSO count does not change.** An uncredited contact is still a QSO and
  still appears in your log; it simply scores nothing. Only the award
  matrix — entities, slots, Challenge points — leaves it out.
- **It is not a worked call, either.** If your only contact with a station was
  an uncredited one, DXCA treats it as unworked and will alert on it again.
  That is the behaviour you want: you still need a contact that counts.
- **`K9HEI/KH9` and `KH9/K9HEI` are the same operation.** cty.xml writes the
  location prefix on whichever side the operator used, so the whitelist is
  matched both ways round. Without that, a confirmed Wake Island QSO was
  rejected — and a rejected contact that carries a QSL is the reliable sign of
  a false rejection, since a QSL match means the operation was real.

**The whitelist lags reality, by design.** A DXpedition ClubLog has not listed
yet is not credited — correctly, in that ClubLog will not credit it either,
but it means a brand-new entity can read as un-worked until the next cty.xml
refresh catches up. If you have just worked something rare and DXCA still
shows it as needed, that is usually why.

**Every uncredited contact is named at refresh time.** Both rules are
otherwise silent — the QSO is simply absent from the totals, and the only
symptom is a number that disagrees with ClubLog's by one. So each ClubLog
refresh logs what it dropped, with the date you need to find the QSO:

```
dxca: user 1: 1 contact(s) in this log earn no DXCC credit:
dxca: user 1:   ZL8AC 20250712 101500Z 40M FT8 DXCC 133 — not on the entity's whitelist, no credit
```

Nothing is printed when there is nothing to report, which is the usual case.
Read it with `journalctl -u dxca` on a Pi, or `run.log` on Windows.

This cuts both ways as a check on your log, not just on DXCA. A contact
appearing here that you do not recognise is worth looking up in ClubLog and
deleting — a busted decode that got logged will show up in exactly this list.

Like the deleted list, both depend on cty.xml. A server that has never
downloaded it applies neither, and totals fall back to the older behaviour.

### Health alerts

Two optional Telegram alerts under **Settings › My station › Telegram**, both
**0 = off** and off by default:

| | fires when |
|---|---|
| **No spots (min)** | nothing has arrived from *any* source — decoders and cluster nodes alike — for this long |
| **Node down (min)** | one cluster node has been *disconnected* for this long while the rest carry on |

They catch the failure that otherwise goes unnoticed for weeks: DXCA up, web
GUI answering perfectly, and the feed quietly dead because the decoders were
closed, the radio was off, or a node dropped and never came back.

**What they cannot do, stated plainly: tell you this host has died.** Nothing
running on a dead Pi can send anything, and Telegram needs the internet, so a
connectivity failure silences the alert about the connectivity failure.
Covering that needs a watcher somewhere else — deliberately not built here,
because a small fleet does not warrant central monitoring.

Node health is keyed on the **connection**, never on traffic. A node sitting
connected with no spots is normal for hours — Hamalert and KST2Mac routinely
do — so alerting on silence would cry wolf every quiet afternoon. A dropped
connection is unambiguous.

You get **one message when a condition starts and one when it clears**, never
a repeat in between: a monitor that repeats itself every tick teaches you to
ignore it, and one that stays silent on recovery makes you go and look, which
is the thing it was supposed to save you. A restart forgets what it had
reported, so a still-quiet feed is announced once more a threshold later.

### FlexRadio panadapter

DXCA can put **your alerts** on a Flex panadapter itself, over the SmartSDR
API on TCP **4992**. Set the radio's IP under **Settings › Server › Destinations ›
FlexRadio**.

It is a tab on **Destinations**, with the UDP and MQTT outputs: a radio is
somewhere spots go. The settings are still per-account, so two operators on
one server each point at their own radio.

Each spot is placed with a colour matching the **Spots screen's own palette**,
so a red mark means on the radio what a red row means on screen:

| level | colour | | level | colour |
|---|---|---|---|---|
| New DXCC | red | | ? DXCC | red, dimmed |
| New Band | blue | | ? Band | blue, dimmed |
| New Mode | amber | | ? Mode | amber, dimmed |
| New Slot | orange | | ? Slot | orange, dimmed |

**Spots expire on a ladder**, adjustable per account:

| level | default life | why |
|---|---|---|
| New DXCC | **60 min** | you may be mid-QSO when it appears and still want to find it after |
| New Band / Mode | **15 min** | about as long as you would stay on a band looking |
| everything else | **1 min** | worth knowing only while the station is still calling |

That last floor is what keeps the display usable. New Slot and the four `?`
levels are most of the alert traffic, and giving them twenty minutes paints
the whole band inside an hour — burying the one red mark the feature exists
to show.

**Only alerts are sent, never the whole feed** — and that is the point. The
alert level comes from your ClubLog matrix, which nothing else on the network
has, so this is the one route by which a panadapter can show *New DXCC*
rather than *a spot*. Everything that narrows Telegram narrows this too:
levels, bands, modes, spotter kind, band mask and the cooldown. It is
independent of `telegram_enabled`, so spots on the radio without a phone
buzzing is a perfectly good way to run.

**If something else already feeds the radio, disconnect it first.** A client
pushing every cluster spot plus DXCA pushing alerts means each alert lands
twice.

**More than one radio is allowed.** The tab holds a list rather than a single
address: **Add radio** for each FlexRadio you want marked — a second rig on
the bench, or another SmartSDR instance — and every alert goes to all of
them. Each row carries its own port, so two instances on one machine can each
take a row, and each row's **On** box keeps the address while stopping the
sending. The same address entered twice is sent to once, a blank port and an
explicit 4992 being the same radio. An account that configured its one radio
before the list existed keeps it, adopted into the list on upgrade with
nothing to re-enter. The ExpertSDR3 tab below works the same way, and the two
lists are wholly independent.

Three things worth knowing about the implementation:

- **It is not a broadcast destination.** Those are UDP datagrams; 4992 is a
  TCP session with sequenced commands. A `Format::Flex` would have been a
  configuration row that looks right and does nothing.
- **The connection is drained.** From the moment a client connects the radio
  streams status messages continuously; a writer that never reads fills its
  receive buffer and blocks the radio's sends.
- **No value may contain a space.** The command is space-delimited
  `key=value`, so a comment with a space in it truncates the command and the
  radio parses the rest as nonsense. Comments are underscored and clipped to
  20 characters — and where level and entity will not both fit, the **entity
  wins**, since the colour has already said which level it is.
- **Nothing is sent on connect.** `client program` was tried against a real
  radio and refused (`unknown client program`), so the handshake stays empty.
  `client gui` — the command that claims a station and its slices — is never
  sent, so DXCA cannot capture a slice.

### ExpertSDR3 panorama (TCI)

The same feature for the other radio. DXCA can put **your alerts** on an
ExpertSDR3 panorama over the **TCI** protocol — SunSDR and anything else
ExpertSDR3 drives — on WebSocket **40001**. Set the radio's IP under
**Settings › Server › Destinations › TCI**.

Everything in the FlexRadio section above applies unchanged: the same colour
palette, the same lifetime ladder and its defaults, the same per-account
settings, the same "only alerts, never the whole feed", and the same warning
about a second client already feeding the display. The two are independent in
every direction — a station can run one, the other, both, or neither, and
either without Telegram.

**More than one radio is allowed.** The TCI tab holds a list rather than a
single address: **Add radio** for each ExpertSDR3 you want marked — a second
rig on the bench, or another ExpertSDR3 instance driving its own panorama —
and every alert goes to all of them. Each row carries its own port, so two
instances on one machine can each take a row, and each row's **On** box keeps
the address while stopping the sending, which is how you silence one radio for
an evening without retyping its IP.

Nothing about the wire changes: each radio is its own WebSocket session and
its own set of `SPOT_DELETE` deadlines, exactly as the single radio was. The
same address entered twice is sent to once — a blank port and an explicit
40001 are the same radio — because two entries for one radio would put two
marks on it per alert and race two deletions to remove one of them. An account
that configured its one radio before the list existed keeps it: the address is
adopted into the list on upgrade, switched on, and nothing needs re-entering.

Written against the published spec: *TCI Protocol*, Expert Group LLC, rev
1.0.7 / TCI 2.0, 12 Jan 2024. One command carries a spot:

```text
SPOT:RN6LHF,CW,7100000,16711680,ANY_TEXT;
     \____/ \/ \_____/ \______/ \______/
     call  mode  Hz     ARGB     text
```

Four things differ from the Flex path, and each shaped the implementation:

- **It is a WebSocket, not a raw socket.** TCI "uses a full duplex web socket
  protocol that runs on top of a TCP connection" (§1.4), so the bytes on 40001
  are an HTTP upgrade and then RFC 6455 frames. Writing `SPOT:...;` to a plain
  TCP socket is discarded by the server without a word — the same
  silent-success trap the Flex notes warn about, one layer down. This costs no
  new dependency: `tungstenite` was already in the tree under axum.
- **There is no lifetime field.** `SPOT` has five arguments and that is all of
  them. A spot stays on the panorama until something removes it, so **DXCA
  holds each deadline itself** and sends `SPOT_DELETE:<call>;` when it passes.
  The consequence worth knowing: if DXCA **restarts** in between, whatever is
  on the panorama stays until you clear it in ExpertSDR3. A dropped *link* is
  different — the deadlines survive a reconnect and the deletions go out when
  it comes back, because the spots are the radio's state, not the link's.
- **`:` `,` and `;` are reserved** (§3.1) and one of them inside a value
  truncates the command, exactly as a space does in SmartSDR's. They are
  replaced with spaces. **Spaces themselves are fine** here, which is the
  opposite of Flex — so the text reads as prose, and the level and entity both
  fit rather than one having to win.
- **`SPOT_CLEAR;` is never sent**, not even to tidy up on connect. The server
  synchronises state across every connected client (§3.1), so it would also
  wipe the spots another logger put there. DXCA only ever deletes a spot it
  placed itself.

The connection is drained on every pass, for the Flex reason plus one: the
server pushes sensor notifications whether or not anyone asked, *and*
WebSocket Pings must be Ponged or the peer hangs up.

### Telnet login (optional, off by default)

Set `telnet_interactive = true` and a telnet session can authenticate with
`LOGIN <callsign>` against its DXCA account. **Loggers are unaffected either
way** — a session that never sends `LOGIN` gets the plain spot feed and has
its input ignored, exactly as before, so RUMlog, Logger32 and N1MM+ need no
change.

Once logged in, you can pass **read-only cluster commands** to one upstream
node at a time:

```
LOGIN VU2CPL
Password: ********
SH/NODES              list the nodes; * marks yours
SET/NODE DB0SUE       aim your commands at one
SH/DX 10              forwarded; the reply comes back to you alone
SH/WWV                solar data, likewise
BYE                   disconnect
```

Commands are **canonicalized, then allowlisted** — `sh/dx` is expanded to
`SHOW/DX` and only then judged, because DXSpider lets any command be
abbreviated and a blocklist could never enumerate every spelling. Anything
that is not a known read-only query is refused with the reason: spotting,
node-side filters (DXCA shares one node session with every user, so a filter
would narrow *everyone's* spots), and anything touching the node account.

Query results never enter the spot feed. A `SHOW/DX` reply looks exactly
like live spots but is hours old, so while a query is outstanding its
node's output goes to the operator who asked and to nobody else.

Design and rationale: [`docs/TELNET-INTERACTIVE.md`](docs/TELNET-INTERACTIVE.md).

Telnet is plaintext: the password crosses the LAN in the clear, and your own
terminal echoes it as you type. Fine for a shack LAN, not for a port
reachable from outside it.

### Spot modes

A spot's mode comes from the decoder (WSJT-X and friends always name it) or
from the cluster comment. Nodes that relay **human** spots — DB0SUE, N2WQ —
send free text with no mode field at all, so for those DXCA falls back to
inferring the mode from the frequency against an **IARU Region 3** band
plan, with the digital calling frequencies (14.074 FT8, 7.0475 FT4, JS8,
WSPR…) taking precedence over the broad segments.

An inferred mode is **marked as one**: the Spots table underlines it with a
dotted rule and says so on hover, and the API exposes `mode_inferred` on
every spot. Award slots can rest on these guesses, so they are visible
rather than silent.

Where the frequency is genuinely ambiguous — beacon windows, 60m, above 2m —
nothing is inferred and the mode shows `—`. Such a spot still classifies for
DXCC entity and band (both mode-independent) but is **excluded from New Mode
and New Slot alerts** rather than being assumed digital. Note the limitation:
mode follows the *transmitting* station's band plan, so a Region 1 station
operating phone low in 40m can be inferred wrongly.

### Alert history

**Alerts** lists what actually went to your Telegram — newest first, in
the same row vocabulary as the Spots feed, with the level tint. Before it
existed the fan-out was invisible: a spot that was flagged, narrowed away by
your band/mode chips, held by the per-callsign cooldown, or refused by
Telegram all looked the same from the UI, which is to say silent.

Failed sends are kept and marked, with Telegram's own error on hover — a bad
chat id otherwise fails quietly forever. History is per account, capped at
the last 500 alerts, and the list refreshes every 15 seconds.

### MQTT destinations

Beside the UDP broadcast destinations, Settings › Server has an admin-only
**MQTT destinations** editor — broker host, port, username, password, base
topic and client ID — for feeding a panadapter overlay (FlexRadio, Aether)
or anything else that speaks MQTT.

Each spot is published **twice**, to sibling topics under the base (default
`shack/dxca/spots`, matching the shack's `shack/<service>/…` convention):

| topic | payload |
|---|---|
| `<base>/json` | `{"callsign":"K1JT","frequency_hz":14074000,"band":"20M","mode":"FT8","snr_db":-10,"comment":"FT8 -10 dB","is_cq":true,…}` |
| `<base>/cluster` | `DX de DXCA:  14074.0  K1JT  FT8 -10 dB  1428Z` |

**For a FlexRadio panadapter, the telnet cluster server is the shorter
route.** Aether takes a DX cluster directly, so pointing it at DXCA's telnet
server on **7575** needs no bridge at all — the UDP broadcast formats are the
wrong shape for this (`cluster` is a `DX de` line in a *datagram*, not a
telnet session), and the Flex's own spot API is a TCP session on 4992 that
Aether already owns.

Aether can read MQTT too, but mind the broker's ACL: a subscriber allowed
only its own tree will connect, authenticate and then receive nothing,
because mosquitto's refusal of a subscription is invisible to most clients.
If spots never arrive over MQTT, check the ACL covers the base topic before
suspecting either program.

The JSON is for anything that wants structure — a Node-RED flow reshaping it
for a panadapter — and the cluster line for anything that already parses the
DX-Spider format. Band is derived from the frequency, so a consumer never has
to. Publishing both costs one extra small message and locks nobody out.

Destinations honour the same `sources` allowlist and `unfiltered` flag as the
UDP ones, so MQTT and your logger see one consistent feed. QoS 0: a spot feed
is a live stream, and dropping one when the queue backs up beats stalling the
pipeline on a slow broker.

Two limits worth knowing: **plain MQTT only** (port 1883, optional
username/password — TLS on 8883 would need rumqttc's rustls feature turned
back on in the workspace manifest), and the broker password is stored in
`data/dxca.db` (0600), **never** in `config/dxca.toml`, which is installed
world-readable — the same reasoning that moved the ClubLog API key.

### Blacklisted calls

The admin-only **Blacklist** tab holds one server-wide list of callsigns to
drop. A listed call is discarded in the pipeline **before the spot ring**, so
it is absent from the Spots table, the telnet cluster server, the filtered
UDP destinations and Telegram alerts — for every account at once. It is not
a display filter; the band and mode chips are that.

> **The alert-level chips narrow hard, and will often show nothing.**
> Picking "New DXCC" makes the feed a New-DXCC feed rather than
> "everything, with DXCC highlighted", so spots that are not flagged are
> hidden — selecting every chip does not bring them back. On a busy feed
> this is usually an empty table: the screen holds the last 500 spots,
> which at ~100 spots/min is about five minutes, while genuinely new spots
> arrive a few times an hour. Working as intended, but worth knowing before
> concluding the highlighting has broken. The Telegram alert history in
> **Alerts** is the reliable record of what was new.


Matching is exact and case-insensitive against the **spotted** station's
callsign: `R1ABC` blocks that call and nothing else. Edits take effect on the
next spot, with no restart.

One honest exception: the **verbatim UDP passthrough** forwards decoder
datagrams untouched, before anything is parsed — that is what keeps a
logger's click-to-fill working — so a blocked call inside a WSJT-X decode can
still reach the logger by that path. Cluster spots have no passthrough and
are dropped completely.

### Accounts

The admin-only **Users** tab lists, creates, edits and deletes accounts.
Edit covers callsign, display name, role and password — any subset, and an
admin may edit their own. Deleting an account takes its sessions, ClubLog
settings, alert preferences and worked matrix with it.

Deletion goes all the way down: removing the **last** account is allowed
and returns the server to the first-run setup card, which is how you start
one over. The single refusal is removing — or demoting — the last **admin**
while other accounts remain, because `/api/setup` only re-arms at zero
accounts, so that state would leave users nobody can administer and no way
back through the UI. Promote another admin first, or delete the others.

## License

MIT — see [LICENSE](LICENSE) — except the files derived from the Meridian
project (© the Meridian authors: Basil Thomas W6BT, Vinod VU3ESV, Ram
VU3RDD), which remain under **Apache-2.0** — see
[LICENSE-APACHE](LICENSE-APACHE):

- `crates/dxca-connect/src/dxcluster/` — the DX-cluster telnet client;
- `web-ui/src/app.css`, `web-ui/src/lib/theme.svelte.ts`, and
  `web-ui/src/lib/ThemeSwitcher.svelte` — the web GUI's design system.

DXCA's modifications to those files are marked `// DXCA:` (`DXCA:` in the
stylesheet), and each file carries the note in its own header.
