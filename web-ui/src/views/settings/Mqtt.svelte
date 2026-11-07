<script lang="ts">
  // Settings › Server › MQTT.
  //
  // Its own endpoint and its own Save, not the global config's: these rows
  // carry a broker password, so they live in the 0600 database rather than in
  // config/dxca.toml, which is installed world-readable.
  import { api } from '../../lib/api';
  import { onMount } from 'svelte';
  import HelpTip from '../../lib/HelpTip.svelte';
  import ChipGroup from '../../lib/ChipGroup.svelte';
  // Only for the source NAMES the picker offers — this page still saves to
  // its own endpoint and never writes the global config.
  import { sourceChoices } from '../../lib/serverconfig.svelte';

  let mqtt = $state<any[]>([]);
  let stats = $state<any>(null);
  let message = $state('');
  let error = $state('');
  let busy = $state(false);

  async function load() {
    const r = await api('GET', '/api/mqtt');
    if (r.status === 200) {
      mqtt = r.json.destinations ?? [];
      stats = r.json;
    }
  }

  /// Counters only. Deliberately does NOT touch `mqtt`: that array is bound to
  /// the inputs, so refreshing it on a timer would wipe whatever the operator
  /// was halfway through typing.
  async function loadStats() {
    const r = await api('GET', '/api/mqtt');
    if (r.status === 200) stats = r.json;
  }

  onMount(() => {
    load();
    // Polled with the rest of the status, not only on save — otherwise
    // "published 0, failed 0" sits there until a page reload while spots are
    // in fact flowing.
    const t = setInterval(loadStats, 5000);
    return () => clearInterval(t);
  });

  async function save() {
    busy = true; message = ''; error = '';
    const r = await api('PUT', '/api/mqtt', { destinations: mqtt });
    if (r.status === 200) {
      mqtt = r.json.destinations;
      message = `Saved — ${r.json.connected} destination(s) connecting.`;
      await load();
    } else {
      error = r.json?.error ?? `HTTP ${r.status}`;
    }
    busy = false;
  }

  const add = () =>
    (mqtt = [
      ...mqtt,
      {
        name: '', host: '192.168.1.169', port: 1883, username: '', password: '',
        topic: 'shack/dxca/spots', client_id: 'dxca',
        sources: [], unfiltered: false, enabled: true, status: false,
      },
    ]);

  const dropRow = (i: number) => (mqtt = mqtt.filter((_, idx) => idx !== i));
</script>

<div class="card">
  <h2>
    MQTT destinations
    <HelpTip label="MQTT destinations">
      <span class="para">
        Each spot is published twice, to sibling topics under the base:
        <code>&lt;topic&gt;/json</code> carries the structured spot (callsign,
        frequency, band, mode, SNR, comment) and
        <code>&lt;topic&gt;/cluster</code> carries the plain DX-cluster line.
      </span>
      <span class="para">
        Plain MQTT on 1883 with optional username/password — TLS is not built
        in. Credentials are stored in <code>data/dxca.db</code> (0600), never in
        <code>config/dxca.toml</code>, which is why this page has its own Save.
      </span>
      <span class="para">
        <b>Sources</b>: <b>All</b> publishes every source; pick names to
        publish only those.
      </span>
      <span class="para">
        <b>Status</b> also publishes two <i>retained</i> topics every minute,
        for dashboards such as Home Assistant:
        <code>&lt;topic&gt;/status</code> (cluster nodes online, spots per
        source) and <code>&lt;topic&gt;/status/&lt;CALL&gt;</code> for each
        account with a ClubLog log (DXCC worked/confirmed, slots, awards,
        the last five alerts). Those are every account's log totals, sent to
        this broker — which is why it is off until ticked.
      </span>
    </HelpTip>
  </h2>
  <div class="editor-scroll">
    <table class="editor">
      <thead>
        <tr>
          <!-- Sources on their own line under each row, as on the UDP tab
               and for the same reason: as a column they were the widest
               thing in the table, and pushed Unf / On / ✕ off the edge. -->
          <th>Name</th><th>Broker</th><th>Port</th><th>User</th><th>Password</th>
          <th>Base topic</th><th>Client ID</th>
          <th>Unf</th><th>Status</th><th>On</th><th></th>
        </tr>
      </thead>
      <tbody>
        {#each mqtt as d, i}
          {@const opts = sourceChoices(d.sources ?? [])}
          <tr>
            <td><input bind:value={d.name} /></td>
            <td><input bind:value={d.host} class="host" /></td>
            <td><input type="number" bind:value={d.port} class="port" /></td>
            <td><input bind:value={d.username} class="port" /></td>
            <td><input type="password" bind:value={d.password} class="port" /></td>
            <td><input bind:value={d.topic} class="host" /></td>
            <td><input bind:value={d.client_id} class="port" /></td>
            <td><input type="checkbox" bind:checked={d.unfiltered} title="Unfiltered: bypass dedupe" /></td>
            <td><input type="checkbox" bind:checked={d.status} title="Status: retained cluster health + per-account totals and recent alerts under <topic>/status" /></td>
            <td><input type="checkbox" bind:checked={d.enabled} /></td>
            <td><button class="drop" title="Remove" onclick={() => dropRow(i)}>✕</button></td>
          </tr>
          <tr class="srcrow">
            <td colspan="11">
              <ChipGroup
                label="Sources"
                options={opts}
                bind:selected={
                  () => new Set(d.sources ?? []),
                  (v) => (d.sources = opts.map((o) => o.key).filter((k) => v.has(k)))
                }
              />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <div class="actions">
    <button onclick={add}>+ Add MQTT destination</button>
    <button class="primary" onclick={save} disabled={busy}>Apply &amp; save</button>
    {#if stats}
      <span class="hint">published {stats.sent ?? 0}, failed {stats.failed ?? 0}</span>
    {/if}
  </div>
  {#if message}<p class="ok">{message}</p>{/if}
  {#if error}<p class="err">{error}</p>{/if}
</div>

<style>
  p {
    margin: 0.75rem 0 0;
  }
</style>
