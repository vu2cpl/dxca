# Multi-station DXCA — moving sources, nodes and outputs to the user

**Status: BUILT, TESTED, AND WITHDRAWN (2026-08-30). Do not rebuild it.**

All of it worked — `feeds_json`, callsign namespacing, `Spot::owner`, the
aggregate, the per-account endpoint, the accounts-driven pipeline — and it was
deployed on noderedpi4 and verified against a copy of production. It was then
removed, because the deployment model does not need it:

> Admin is the main user. Others are all guest users. All sources and local
> network settings to be only with admin. Guests can only login, set their
> ClubLog credentials, and select the spots they want their Telegram to alert
> on. — Manoj, 2026-08-30

**No guest owns a source, a node or an output**, so there is nothing to own
per account. One station's feeds, which is what `config/dxca.toml` was already
doing. The architecture solved a problem this deployment does not have, and
keeping it would have meant carrying a schema column, a namespacing scheme and
a second config endpoint to support a case that does not exist.

Three pieces were kept because they stand alone:

* the `apply_sources` fix freeing a retiring listener that holds a port an
  addition wants — without it, renaming a source while keeping its port fails
  with `EADDRINUSE` against ourselves;
* the telnet first-bytes log, which answered this module's long-standing open
  question — **RUMlog sends nothing at all on connect**, so a callsign prompt
  would need the client to answer one, which is a separate experiment;
* the `format.rs` test recording that `source_name` reaches the wire as the
  spotter callsign, and that punctuation in it is welded rather than rejected.

**If this is ever revisited**, the findings below are the valuable part — in
particular the reason namespacing must not touch `source_name`, and the fact
that the config file stops controlling what runs the moment accounts own
feeds. Read it before writing anything.

The design below is left as it was written.

## The model

Today DXCA is a *station's* server that happens to have accounts on it.
Sources, nodes and destinations are server-wide; accounts carry only a
ClubLog log and alert preferences. The five installs in the field are five
copies of that, one operator each.

The model this note describes is different: **one server, many stations.**

> Admin is only to maintain the Pi/server. Individual users or callsigns are
> different stations and decide what nodes they want to log into and ingest.
> Only LoTW data and ClubLog are of common nature. Other users decide their
> sources and destinations. — Manoj, 2026-08-30

And, decisively for the internals:

> It's going to be only one stream of spots, but filtered and displayed or
> alerted in a different user-level way.

So ingestion stays **one pipeline**. What moves is *ownership*: each account
owns its sources, its nodes and its outputs, and sees the stream through the
window of what it owns.

| | owner | why |
|---|---|---|
| cty.xml, LoTW list, ClubLog API key, blacklist | **admin** | one copy, every account classifies against it (the API key is baked into a released binary, so an admin need not supply one) |
| user accounts | **admin** | account management |
| **passthrough destinations** | **admin** | see [Passthrough](#passthrough-stays-server-wide) |
| UDP sources | **user** | that station's decoders |
| cluster nodes | **user** | that station's logins, under its own callsign |
| UDP + MQTT destinations | **user** | that station's loggers |
| ClubLog log, alerts, Telegram, Flex | **user** | already is |

## The finding that shapes everything: the name is the key

Both hot-apply paths diff by **name**, and the name is load-bearing far
beyond configuration:

- `PipelineState::apply_sources` keys its listener map on `(name, port)`
- `NodeManager::apply` keys `clients` and `statuses` on `name`
- every spot carries `source_name` — a decoder name *or* a node name, one
  namespace (README, *Who spotted it*)
- `/api/status` reports `spots_per_source` and `cluster_nodes` by name
- destinations filter on `allowed_sources`, by name

So the moment two accounts both name a node `N2WQ-2`, they collide — in the
client map, in the status map, and in every spot either produces.

**Proposal: namespace internally, show bare names to the owner.** Store the
key as `<callsign>:<name>` — `VU2CPL:N2WQ-2` — and display only `N2WQ-2` on
that operator's screens. Two consequences, both wanted:

1. Collisions become impossible without asking operators to coordinate names
   across stations that have never met.
2. **Ownership becomes derivable from the spot itself.** `source_name` already
   travels with every spot through the pipeline, the ring, the telnet feed and
   the destinations. Prefixing it means a user's filter is a prefix test, with
   no second lookup and no new field on `Spot`.

### REVISED 2026-08-30, before writing the aggregation

**The qualified name must not reach `Spot::source_name`.** Probing the first
consumer found why: `format::format` uses `source_name` as the **spotter
callsign** on the DX cluster line, and filters it to `[A-Z0-9/-]`. A colon is
not in that set, so `VU2CPL:MSHV` does not error and does not truncate — it
becomes `VU2CPLMSHV`, and `DX de VU2CPLMSHV:` goes out to every logger. It
looks like a plausible callsign, so nobody would notice it was wrong.

That is one consumer. The others are `spots_per_source`, the Spots table's
Source column, the destination allowlist, `send_raw`'s passthrough keying and
the status page. "Qualify everywhere and strip at each boundary" needs every
one of them to be right, and this one proves the failure is silent.

**So namespacing stays inside the config and apply layer**, where uniqueness
is actually needed — the listener map, the client map, the status map. When a
spot is produced the qualified key is split: `source_name` stays bare, and
the owner goes in a new `Spot` field of its own.

Ownership is still derivable from the spot, which was the point; it just
travels in its own field instead of smuggled inside a string. The per-user
filter becomes `spot.owner == callsign` — explicit, and impossible to leak
onto the wire. A regression test in `format.rs` records the trap.

The original reasoning, kept because the collision problem it solves is real:

That last point is what makes "one stream, filtered per user" nearly free:
the filter is already plumbed everywhere, it just has nothing to key on yet.

## What has to change

### Aggregate, then apply

`apply_sources` and `NodeManager::apply` already diff correctly against a
desired list. Neither needs rewriting — they need a different *caller*.
Instead of `cfg.udp_sources` and `cfg.cluster_nodes`, build the wanted list by
walking every account and concatenating theirs, namespaced. Add, remove or
re-point a user's node and the same diff starts or retires exactly that
session.

### UDP ports must be unique across accounts

One socket per port; DXCA is the sole listener. Two stations both choosing
2333 is a bind failure, and `apply_sources` binds additions before retiring
anything precisely so it surfaces as an error rather than dying in a task.
Under per-user ownership that error would arrive on *someone else's* save, so
the check has to move earlier: **validate the port is free across all
accounts at save time**, and reject with the owning callsign named.

### Node logins do not collide

Each station logs in under its own callsign, so two accounts configuring the
same host are two sessions, not a duplicate login that DXSpider would kick.
No sharing, no coordination.

Worth sizing rather than assuming: five stations × nine nodes is 45 telnet
sessions from one IP, and some nodes cap connections per address. If that
bites, the fallback is sharing one session when host, port *and* login
callsign all match — but do not build that until a node actually complains.

### Storage moves out of `config/dxca.toml`

`udp_sources` and `cluster_nodes` move to the per-account row, joining
ClubLog, notify and Flex. The same argument that moved MQTT applies: the file
installs 0644 while `data/dxca.db` is 0600, and a node password has no
business in a world-readable file.

What stays in the TOML: `web_bind`, `telnet_port`, ports and paths, the
refresh intervals, and the passthrough destinations.

### Outputs

Per-account UDP and MQTT destinations, dispatched where the alert fan-out
already dispatches Telegram and Flex — `fan_out` is per-user by construction,
so this is two more sinks in a loop that exists.

**But outputs are a feed, not alerts.** `broadcast_spot` fires for every
deduped spot; `fan_out` only runs for spots that pass a user's *alert*
filters. A logger wants the feed. So per-user outputs need their own
narrowing — which sources, bands and modes reach my logger — separate from
the alert ladder. Simplest coherent answer: outputs follow the account's
**source ownership** plus an optional band/mode narrowing of their own.

## Passthrough stays server-wide

`send_raw` relays a decoder's datagram **verbatim, before parsing**, which is
what keeps RUMlog's click-to-fill working. It is keyed to a source, not a
user, and under one shared stream it has no obvious owner.

Manoj's call, 2026-08-30: **leave it admin-owned.** It is the one output that
is genuinely about the server's own machine rather than about a station.

## The practical split (Manoj, 2026-08-30)

The general model above is right, but the deployment is narrower and the UI
follows the narrower one:

> Admin is assumed to be on the local network. All other guest users are
> configured only for spot alerts and ingestion, so only the admin is going
> to use UDP sources, RUMlog and so on.

So:

| | who sees it | why |
|---|---|---|
| **Cluster nodes** | every account | the one feed a guest owns — what they ingest and are alerted on |
| ClubLog, alerts, Telegram, FlexRadio | every account | already per-account |
| **UDP sources** | admin only | the decoders are on the admin's LAN; a guest has none |
| **Spot outputs**, passthrough | admin only | they feed the admin's loggers |
| Reference data, Users | admin only | unchanged |

**The storage does not change for this.** Sources and outputs are still owned
by an account — the admin's — and still namespaced. Only the *page* is
admin-gated. That keeps one mechanism rather than two, and leaves the door
open if a guest ever does want their own decoder.

## Operational consequence: the TOML stops disabling things

Once an account owns feeds, **editing `config/dxca.toml` no longer changes
what runs.** Sources and nodes come from the database; the file keeps only
`web_bind`, ports, paths, refresh intervals and the passthrough rows.

This is not theoretical. It was found the hard way: a dry run against a copy
of production was made "safe" by setting `enabled = false` on every node in
the TOML — and it dialled all nine production nodes anyway, because the
aggregate had already stopped reading the file. The habit of disabling a node
by editing the config is now a no-op that looks like it worked.

Two practical rules follow:

* **Disable in the UI, not the file** — or, for a local run against a copy of
  production, disable in the account's `feeds_json` and prove it with
  `lsof -a -p PID -iTCP -sTCP:ESTABLISHED`, which should show zero.
* **The startup banner prints from the file**, so it says `nodes []` while
  nine are running. That line needs to print the effective set; until it
  does, do not read it as evidence.

## Open questions

1. **What does the telnet server serve?** Today every connected logger gets
   the same feed. With per-user ownership, should a logger get its owner's
   window? `LOGIN <callsign>` already exists and would stop being optional —
   but that changes behaviour for RUMlog, Logger32 and N1MM+, which connect
   without logging in and must keep working.
2. **What does a user see before they own anything?** A new account with no
   sources and no nodes sees an empty Spots screen. Correct, but it looks
   broken. The first-run path needs an answer.
3. **Does the shared spot ring stay shared?** `pipeline.spots` is one
   VecDeque serving `/api/spots`. Filtering per request is cheap and keeps one
   buffer; the alternative is per-user rings, which is memory for no obvious
   gain. Filtering looks right, but it means the ring holds spots a user will
   never see.
4. **Blacklist scope.** Currently server-wide and admin-owned. Left there by
   default, but a station may want its own.

## Migration

Every install in the field has exactly one account, which is what makes this
tractable: on first start after the upgrade, move the TOML's `udp_sources`
and `cluster_nodes` into the sole account's row, namespaced under its
callsign, and leave `broadcast_destinations` split — passthrough rows stay in
the file, the rest move to that account.

With more than one account and no way to guess ownership, refuse and require
an admin to assign them. That case does not exist yet in the field and should
not be guessed at.

**Rollback is the previous binary plus the untouched TOML**, which is why the
migration must not delete the moved sections from the file until an admin
confirms — write the new state, leave the old, and clear it on a later
version.
