<script lang="ts">
  // Settings › Server › Reference data — the server-wide lists every account is
  // read against, plus the server's own facts.
  //
  // The call blacklist is here rather than under Access because it is the same
  // KIND of thing as cty.xml and the LoTW list: one list, server-wide, that
  // every account is subject to whether or not they know it exists. Access is
  // about who may log in, which the blacklist has nothing to do with.
  //
  // Both are one file backing one in-memory structure that every account
  // shares, so both are admin-only and both refresh server-wide — unlike a
  // ClubLog log, which is per account. They live together because that
  // symmetry is the point.
  import { api, ago } from '../../lib/api';
  import { onMount } from 'svelte';
  import { status, refreshStatus } from '../../lib/status.svelte';
  import HelpTip from '../../lib/HelpTip.svelte';
  import ApplySave from '../../lib/ApplySave.svelte';
  import ConfigGate from '../../lib/ConfigGate.svelte';
  import Blacklist from '../Blacklist.svelte';
  import { server, loadServerConfig } from '../../lib/serverconfig.svelte';

  let s = $derived(status());

  /// A refresh interval as the file-only line shows it. `0` means the job is
  /// switched off, and saying "0d" would read as "constantly" — the exact
  /// opposite of what it does.
  const days = (n: number) => (n ? `${n}d` : 'off');
  let message = $state('');
  let error = $state('');
  let busy = $state(false);

  onMount(() => {
    loadServerConfig();
    refreshStatus();
  });

  async function refreshCty() {
    busy = true; message = 'Downloading cty.xml from ClubLog…'; error = '';
    const r = await api('POST', '/api/cty/refresh');
    busy = false;
    if (r.status === 200) {
      message = `cty.xml refreshed: ${r.json.cty_entities} entities.`;
      await refreshStatus();
      // The download rewrites the stored timestamp, so the config we are
      // showing is stale the moment it succeeds.
      await loadServerConfig(true);
    } else { message = ''; error = r.json?.error ?? `HTTP ${r.status}`; }
  }

  async function refreshLotw() {
    busy = true; message = 'Downloading LoTW users list…'; error = '';
    const r = await api('POST', '/api/lotw/refresh');
    busy = false;
    if (r.status === 200) {
      message = `LoTW list refreshed: ${r.json.lotw_users} users.`;
      await refreshStatus();
      await loadServerConfig(true);
    } else { message = ''; error = r.json?.error ?? `HTTP ${r.status}`; }
  }

  async function refreshIota() {
    busy = true; message = 'Downloading IOTA directory…'; error = '';
    const r = await api('POST', '/api/iota/refresh');
    busy = false;
    if (r.status === 200) {
      message = `IOTA directory refreshed: ${r.json.iota_groups} groups.`;
      await refreshStatus();
      await loadServerConfig(true);
    } else { message = ''; error = r.json?.error ?? `HTTP ${r.status}`; }
  }

  async function refreshFcc() {
    busy = true;
    message = 'Downloading the FCC amateur database — ~200 MB, this takes minutes…';
    error = '';
    const r = await api('POST', '/api/fcc/refresh');
    busy = false;
    if (r.status === 200) {
      message = `FCC table refreshed: ${r.json.fcc_calls} calls.`;
      await refreshStatus();
      await loadServerConfig(true);
    } else { message = ''; error = r.json?.error ?? `HTTP ${r.status}`; }
  }
</script>

{#if s}
  <div class="card">
    <h2>Server</h2>
    <dl class="stats">
      <div><dt>Version</dt><dd class="mono">v{s.version}</dd></div>
      <div><dt>Users</dt><dd class="num">{s.users}</dd></div>
      <div><dt>TCP clients</dt><dd class="num">{s.telnet_clients}</dd></div>
      <div><dt>UDP sent</dt><dd class="num">{s.udp_sent}</dd></div>
      <div>
        <dt>UDP failed</dt>
        <dd class="num" class:err={s.udp_failed}>{s.udp_failed}</dd>
      </div>
    </dl>
  </div>
{/if}

<ConfigGate>
  <div class="card">
    <h2>Reference data — shared by all users</h2>

    <!--
      The API key sits behind a disclosure because a released dxca ships with
      one (baked in at build time), which makes this field a leftover for
      almost every server — and a visible empty password box mostly invites
      someone to paste the wrong secret into it. It stays reachable for the
      three cases that still need it: a build made without a key, a shipped key
      ClubLog ever revokes, and admins who would rather spend their own quota.
      Open by default when there is no built-in key, because then it is the
      only way to get a country file at all.
    -->
    <details class="advanced" open={!server.cfg.clublog_key_built_in}>
      <summary>
        Advanced
        {#if server.cfg.clublog_api_key}
          <span class="tag">own API key set</span>
        {:else if !server.cfg.clublog_key_built_in}
          <span class="tag warn">API key needed</span>
        {/if}
      </summary>

      <div class="settings-form">
        <span class="label">
          ClubLog API key
          <HelpTip label="ClubLog API key">
            Fetches <b>cty.xml</b>, the DXCC prefix database every account is
            classified against — so it belongs to the server, not to an operator.
            It is <b>not</b> used to download anyone's log; that uses each user's
            own email and app password under <b>My station › ClubLog account</b>.
            {#if server.cfg.clublog_key_built_in}
              This build ships with a key, so you can leave this empty; anything
              you enter here is used instead of it.
            {:else}
              This build has no key of its own, so cty.xml cannot download until
              you enter one.
            {/if}
          </HelpTip>
        </span>
        <input
          type="password"
          bind:value={server.cfg.clublog_api_key}
          placeholder={server.cfg.clublog_key_built_in
            ? 'optional — this build has a key of its own'
            : 'required — from clublog.org, one key for the whole server'}
        />
      </div>
    </details>

    <!-- Two shared datasets, three columns: what, when, act. -->
    <table class="refdata">
      <tbody>
        <tr>
          <td class="what">cty.xml<br /><span class="hint">{s?.cty_entities ?? '—'} entities</span></td>
          <td class="when hint">
            {#if server.cfg.read_only.cty_refresh_days}
              auto every {server.cfg.read_only.cty_refresh_days}d ·
            {:else}
              auto off ·
            {/if}
            {#if server.cfg.cty_last_refresh_unix}
              last {ago(server.cfg.cty_last_refresh_unix)} ago
            {:else}
              never downloaded here
            {/if}
          </td>
          <td>
            <button onclick={refreshCty} disabled={busy || !server.cfg.clublog_api_key}>Refresh now</button>
          </td>
        </tr>
        <tr>
          <td class="what">LoTW users<br /><span class="hint">{s?.lotw_users ?? '—'} calls</span></td>
          <td class="when hint">
            {#if server.cfg.read_only.lotw_refresh_days}
              auto every {server.cfg.read_only.lotw_refresh_days}d ·
            {:else}
              auto off ·
            {/if}
            {#if server.cfg.lotw_last_refresh_unix}
              last {ago(server.cfg.lotw_last_refresh_unix)} ago
            {:else}
              never downloaded here
            {/if}
          </td>
          <td><button onclick={refreshLotw} disabled={busy}>Refresh now</button></td>
        </tr>
        <tr>
          <td class="what">
            IOTA directory<br />
            <span class="hint">{s?.iota_groups || '—'} groups</span>
          </td>
          <td class="when hint">
            {#if server.cfg.read_only.iota_refresh_days}
              auto every {server.cfg.read_only.iota_refresh_days}d ·
            {:else}
              auto off ·
            {/if}
            {#if server.cfg.iota_last_refresh_unix}
              last {ago(server.cfg.iota_last_refresh_unix)} ago
            {:else}
              never downloaded here
            {/if}
          </td>
          <td>
            <button
              onclick={refreshIota}
              disabled={busy}
              title="groups.json from iota-world.org (~290 KB) — validates spot IOTA references and names the island groups. Their terms are personal non-commercial use, which is why it downloads here rather than shipping with DXCA."
              >Refresh now</button
            >
          </td>
        </tr>
        <tr>
          <td class="what">
            FCC call→state<br />
            <span class="hint">{s?.fcc_calls || '—'} calls</span>
          </td>
          <td class="when hint">
            {#if server.cfg.read_only.fcc_refresh_days}
              auto every {server.cfg.read_only.fcc_refresh_days}d ·
            {:else}
              auto off ·
            {/if}
            {#if server.cfg.fcc_last_refresh_unix}
              last {ago(server.cfg.fcc_last_refresh_unix)} ago
            {:else}
              never downloaded here — State alerts stay quiet until it is
            {/if}
          </td>
          <td>
            <button
              onclick={refreshFcc}
              disabled={busy}
              title="The FCC amateur database (~200 MB download, distilled here to ~8 MB) — which US state a call is licensed in, the data behind New State / ? State. The schedule only re-runs after this first manual pull; a licensee operating away from their license address will still read as their license state."
              >Download now</button
            >
          </td>
        </tr>
      </tbody>
    </table>
    {#if message}<p class="ok">{message}</p>{/if}
    {#if error}<p class="err">{error}</p>{/if}

    <ApplySave />

    <!-- Everything in config/dxca.toml that this page cannot change, listed
         so the UI never implies it owns a setting it does not. It must stay
         COMPLETE: the IOTA and FCC intervals were added to the config in
         2.17.0 and left off this line until 2.17.4, which made the line
         quietly wrong rather than merely terse. The telnet login flag is
         here for a stronger reason — it changes what port 7575 accepts, and
         it was previously invisible from the web UI altogether. -->
    <p class="hint file-only">
      File-only settings: web {server.cfg.read_only.web_bind} · telnet
      {server.cfg.read_only.telnet_port}
      (login {server.cfg.read_only.telnet_interactive ? 'on' : 'off'}) ·
      dedupe {server.cfg.read_only.dedupe_window_secs}s · ring
      {server.cfg.read_only.spot_ring_capacity} · refresh: cty
      {days(server.cfg.read_only.cty_refresh_days)}, LoTW
      {days(server.cfg.read_only.lotw_refresh_days)}, IOTA
      {days(server.cfg.read_only.iota_refresh_days)}, FCC
      {days(server.cfg.read_only.fcc_refresh_days)} · data dir
      <code>{server.cfg.read_only.data_dir}</code> (edit config/dxca.toml + restart).
    </p>
  </div>
</ConfigGate>

<Blacklist />

<style>
  /* Label over value, wrapping into as many columns as fit. */
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 0.9rem 2rem;
    margin: 0;
  }

  .stats dt {
    font-size: var(--fs-hint);
    color: var(--muted);
  }

  .stats dd {
    margin: 0.1rem 0 0;
    font-size: 1.05rem;
  }

  .stats dd.num {
    font-variant-numeric: tabular-nums;
  }

  .stats dd.err {
    color: var(--err);
  }

  .refdata {
    width: auto;
    margin-top: 0.75rem;
  }

  .refdata td {
    padding: 0.35rem 1.25rem 0.35rem 0;
    vertical-align: middle;
  }

  .refdata .what {
    line-height: 1.35;
  }

  .refdata .when {
    white-space: nowrap;
  }

  .file-only {
    margin: 0.75rem 0 0;
    line-height: 1.5;
  }

  p {
    margin: 0.75rem 0 0;
  }

  /* The API-key disclosure. Deliberately quiet: on a server with a built-in
     key this is a row almost nobody should open. */
  .advanced > summary {
    cursor: pointer;
    font-size: var(--fs-hint);
    color: var(--muted);
    user-select: none;
  }

  .advanced[open] > summary {
    margin-bottom: 0.5rem;
  }

  .advanced .tag {
    margin-left: 0.4rem;
    font-size: var(--fs-hint);
  }

  .advanced .tag.warn {
    color: var(--warn);
  }
</style>
