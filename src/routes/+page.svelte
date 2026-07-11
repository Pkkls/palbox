<script>
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";

  let docker = $state({ installed: false, running: false, version: "", message: "Checking Docker…" });
  let runtime = $state({ exists: false, running: false, ready: false, imageReady: false, memory: "" });
  let tunnel = $state({ running: false, claimUrl: "", address: "", message: "" });
  let settings = $state(null);
  let players = $state([]);
  let logs = $state("");
  let tab = $state("overview");
  let busy = $state("");
  let toast = $state("");
  let pulling = $state(false);
  let ready = $state(false);

  const dockerReady = $derived(docker.installed && docker.running);
  const stage = $derived(
    !ready ? "loading" : !dockerReady ? "docker" : !runtime.exists ? "setup" : "dashboard"
  );

  function notify(msg) {
    toast = msg;
    setTimeout(() => (toast = ""), 3200);
  }

  function randomPassword() {
    const bytes = new Uint8Array(9);
    crypto.getRandomValues(bytes);
    return "adm-" + btoa(String.fromCharCode(...bytes)).replace(/[^a-zA-Z0-9]/g, "").slice(0, 10);
  }

  async function refreshStatus() {
    try {
      docker = await invoke("check_docker");
      if (dockerReady) {
        runtime = await invoke("server_status");
        tunnel = await invoke("tunnel_status");
      }
    } catch (e) {
      notify(String(e));
    }
  }

  async function loadSettings() {
    settings = await invoke("load_settings");
    if (!settings.adminPassword) settings.adminPassword = randomPassword();
  }

  onMount(async () => {
    await loadSettings();
    await refreshStatus();
    ready = true;
    const id = setInterval(async () => {
      await refreshStatus();
      if (stage === "dashboard" && runtime.running && tab === "players") await loadPlayers();
    }, 4000);
    return () => clearInterval(id);
  });

  async function run(label, fn) {
    busy = label;
    try {
      await fn();
    } catch (e) {
      notify(String(e));
    } finally {
      busy = "";
      await refreshStatus();
    }
  }

  async function createServer() {
    if (!runtime.imageReady) {
      pulling = true;
      try {
        await invoke("pull_image");
      } catch (e) {
        pulling = false;
        notify("Image download failed: " + e);
        return;
      }
      pulling = false;
    }
    await invoke("save_settings", { settings });
    await invoke("create_server", { settings });
    notify("Server created. First run downloads the game — this takes a few minutes.");
    // The tunnel is turned on separately from the Safe access panel, once a
    // playit key is set — starting it here would fail before setup and look
    // like the server creation itself failed.
    await refreshStatus();
  }

  async function saveAndApply() {
    await invoke("save_settings", { settings });
    if (runtime.exists) {
      await invoke("create_server", { settings });
      notify("Settings applied — server recreated.");
    } else {
      notify("Settings saved.");
    }
  }

  async function loadPlayers() {
    try {
      players = await invoke("list_players");
    } catch {
      players = [];
    }
  }

  async function kick(p) {
    await run("kick", () => invoke("kick_player", { steamid: p.steamid }));
    notify(`Kicked ${p.name}`);
  }
  async function ban(p) {
    await run("ban", () => invoke("ban_player", { steamid: p.steamid }));
    notify(`Banned ${p.name}`);
  }

  let broadcastMsg = $state("");
  async function sendBroadcast() {
    if (!broadcastMsg.trim()) return;
    await run("broadcast", () => invoke("broadcast", { message: broadcastMsg }));
    notify("Message sent.");
    broadcastMsg = "";
  }

  async function refreshLogs() {
    logs = await invoke("get_logs", { lines: 300 });
  }

  async function copy(text) {
    await navigator.clipboard.writeText(text);
    notify("Copied to clipboard.");
  }

  $effect(() => {
    if (tab === "logs" && stage === "dashboard") refreshLogs();
    if (tab === "players" && runtime.running) loadPlayers();
  });
</script>

{#if toast}
  <div class="toast">{toast}</div>
{/if}

{#if stage === "loading"}
  <div class="center"><div class="spinner"></div></div>

{:else if stage === "docker"}
  <div class="center">
    <div class="gate">
      <div class="badge warn">Docker required</div>
      <h1>PalBox needs Docker to run your server</h1>
      <p>{docker.message}</p>
      <div class="gate-actions">
        <button class="primary" onclick={() => openUrl("https://www.docker.com/products/docker-desktop/")}>
          Get Docker Desktop
        </button>
        <button onclick={refreshStatus}>Re-check</button>
      </div>
      <p class="fine">Install it once, start it, then come back — PalBox handles everything else.</p>
    </div>
  </div>

{:else if stage === "setup"}
  <div class="center">
    <div class="wizard">
      <div class="badge ok">Docker {docker.version} ready</div>
      <h1>Create your Palworld server</h1>
      <p class="sub">Pick your settings. Friends will join through a safe relay — your IP stays hidden.</p>

      {#if settings}
        <div class="grid">
          <label>Server name<input bind:value={settings.serverName} /></label>
          <label>Max players<input type="number" min="1" max="32" bind:value={settings.players} /></label>
          <label>Join password <span class="hint">optional</span><input bind:value={settings.serverPassword} placeholder="leave empty for none" /></label>
          <label>Admin password<input bind:value={settings.adminPassword} /></label>
          <label>Difficulty
            <select bind:value={settings.difficulty}>
              <option value="None">Normal</option>
              <option value="Casual">Casual</option>
              <option value="Hard">Hard</option>
            </select>
          </label>
          <label>Death penalty
            <select bind:value={settings.deathPenalty}>
              <option value="None">Keep everything</option>
              <option value="Item">Drop items</option>
              <option value="ItemAndEquipment">Drop items + gear</option>
              <option value="All">Drop everything</option>
            </select>
          </label>
        </div>

        <div class="mode">
          <button type="button" class="mode-opt {settings.tunnelMode === 'tunnel' ? 'active' : ''}" onclick={() => (settings.tunnelMode = 'tunnel')}>
            <b>🔒 Safe tunnel</b><span>No port-forward, IP hidden. Recommended.</span>
          </button>
          <button type="button" class="mode-opt {settings.tunnelMode === 'direct' ? 'active' : ''}" onclick={() => (settings.tunnelMode = 'direct')}>
            <b>Direct</b><span>Needs router port-forward; exposes your IP.</span>
          </button>
        </div>

        <button class="primary big" disabled={busy !== '' || pulling} onclick={() => run('create', createServer)}>
          {#if pulling}Downloading server image… (a few minutes){:else if busy === 'create'}Creating…{:else}Create my server{/if}
        </button>
        {#if pulling}<p class="fine">First time only — the ~7&nbsp;GB server image is downloaded once.</p>{/if}
      {/if}
    </div>
  </div>

{:else}
  <!-- dashboard -->
  <div class="app">
    <aside>
      <div class="brand">Pal<span>Box</span></div>
      <nav>
        <button class={tab === "overview" ? "on" : ""} onclick={() => (tab = "overview")}>Overview</button>
        <button class={tab === "players" ? "on" : ""} onclick={() => (tab = "players")}>Players</button>
        <button class={tab === "settings" ? "on" : ""} onclick={() => (tab = "settings")}>Settings</button>
        <button class={tab === "logs" ? "on" : ""} onclick={() => (tab = "logs")}>Logs</button>
      </nav>
      <div class="state">
        <span class="dot {runtime.ready ? 'up' : runtime.running ? 'starting' : 'down'}"></span>
        {runtime.ready ? "Running" : runtime.running ? "Starting…" : "Stopped"}
        {#if runtime.memory}<small>{runtime.memory}</small>{/if}
      </div>
    </aside>

    <main>
      {#if tab === "overview"}
        <header class="head">
          <div>
            <h1>{settings?.serverName}</h1>
            <p class="sub">{runtime.ready ? "Your server is online and joinable." : runtime.running ? "Starting up — first run downloads the game (a few minutes). Hang tight." : "Your server is stopped."}</p>
          </div>
          <div class="controls">
            {#if runtime.running}
              <button disabled={busy !== ''} onclick={() => run('restart', () => invoke('restart_server'))}>Restart</button>
              <button class="danger" disabled={busy !== ''} onclick={() => run('stop', () => invoke('stop_server'))}>Stop</button>
            {:else}
              <button class="primary" disabled={busy !== ''} onclick={() => run('start', () => invoke('start_server'))}>Start</button>
            {/if}
          </div>
        </header>

        <section class="card tunnel">
          <div class="card-title">🔒 Safe access</div>
          {#if tunnel.running && tunnel.address}
            <p>Share this address with your friends — nothing to set up on their side:</p>
            <div class="addr">
              <code>{tunnel.address}</code>
              <button onclick={() => copy(tunnel.address)}>Copy</button>
            </div>
            <button class="ghost" onclick={() => run('tunnel', () => invoke('tunnel_stop'))}>Turn off tunnel</button>
          {:else if tunnel.running}
            <p>{tunnel.message}</p>
            <button class="ghost" onclick={() => run('tunnel', () => invoke('tunnel_stop'))}>Turn off tunnel</button>
          {:else}
            <p>Friends join through a relay, so no port is opened and your public IP stays hidden. This needs a free <b>playit.gg</b> key, a one-time setup. <button class="link" onclick={() => openUrl('https://github.com/Pkkls/palbox/blob/main/docs/TUNNEL-SETUP.md')}>Full step-by-step guide</button></p>
            <ol class="steps">
              <li>Get a free Docker agent key at <button class="link" onclick={() => openUrl('https://playit.gg/account/agents/new-docker')}>playit.gg</button>, and add a <b>UDP tunnel</b> whose local address is <code>127.0.0.1:{settings?.port ?? 8211}</code>.</li>
              <li>Paste the secret key here:
                <div class="row">
                  <input placeholder="playit secret key" bind:value={settings.playitSecret} />
                  <button onclick={() => run('savekey', () => invoke('save_settings', { settings }))}>Save</button>
                </div>
              </li>
            </ol>
            <button class="primary" disabled={busy !== '' || !settings?.playitSecret} onclick={() => run('tunnel', () => invoke('tunnel_start'))}>Turn on safe tunnel</button>
          {/if}
        </section>

        <section class="card">
          <div class="card-title">Announce to players</div>
          <div class="row">
            <input placeholder="Message shown in-game…" bind:value={broadcastMsg} />
            <button disabled={busy !== '' || !runtime.running} onclick={sendBroadcast}>Send</button>
            <button disabled={busy !== '' || !runtime.running} onclick={() => run('save', () => invoke('save_world'))}>Save world</button>
          </div>
        </section>
      {/if}

      {#if tab === "players"}
        <header class="head"><div><h1>Players</h1><p class="sub">{players.length} online</p></div>
          <button disabled={!runtime.running} onclick={loadPlayers}>Refresh</button></header>
        {#if !runtime.running}
          <p class="empty">Start the server to see players.</p>
        {:else if players.length === 0}
          <p class="empty">No one is connected right now.</p>
        {:else}
          <table>
            <thead><tr><th>Name</th><th>Steam ID</th><th></th></tr></thead>
            <tbody>
              {#each players as p}
                <tr>
                  <td>{p.name}</td>
                  <td class="mono">{p.steamid}</td>
                  <td class="right">
                    <button onclick={() => kick(p)}>Kick</button>
                    <button class="danger" onclick={() => ban(p)}>Ban</button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      {/if}

      {#if tab === "settings" && settings}
        <header class="head"><div><h1>Settings</h1><p class="sub">Applying changes recreates the server (saves are kept).</p></div></header>
        <section class="card">
          <div class="grid">
            <label>Server name<input bind:value={settings.serverName} /></label>
            <label>Max players<input type="number" min="1" max="32" bind:value={settings.players} /></label>
            <label>Join password<input bind:value={settings.serverPassword} placeholder="none" /></label>
            <label>Admin password<input bind:value={settings.adminPassword} /></label>
            <label>Difficulty
              <select bind:value={settings.difficulty}>
                <option value="None">Normal</option><option value="Casual">Casual</option><option value="Hard">Hard</option>
              </select>
            </label>
            <label>Death penalty
              <select bind:value={settings.deathPenalty}>
                <option value="None">Keep everything</option><option value="Item">Drop items</option>
                <option value="ItemAndEquipment">Drop items + gear</option><option value="All">Drop everything</option>
              </select>
            </label>
            <label>EXP rate<input type="number" step="0.1" min="0.1" bind:value={settings.expRate} /></label>
            <label>Capture rate<input type="number" step="0.1" min="0.1" bind:value={settings.palCaptureRate} /></label>
          </div>
          <label class="check"><input type="checkbox" bind:checked={settings.pvp} /> Enable PvP</label>
          <label class="check"><input type="checkbox" bind:checked={settings.autoUpdate} /> Auto-update server on boot</label>
          <label class="check"><input type="checkbox" bind:checked={settings.backupEnabled} /> Automatic backups</label>
          <div class="row end">
            <button class="danger ghost" onclick={() => run('delete', () => invoke('delete_server'))}>Delete server</button>
            <button class="primary" disabled={busy !== ''} onclick={() => run('apply', saveAndApply)}>Save &amp; apply</button>
          </div>
        </section>
      {/if}

      {#if tab === "logs"}
        <header class="head"><div><h1>Logs</h1><p class="sub">Last 300 lines</p></div>
          <button onclick={refreshLogs}>Refresh</button></header>
        <pre class="logs">{logs || "No logs yet."}</pre>
      {/if}
    </main>
  </div>
{/if}

<style>
  :global(:root) {
    --bg: #f4f6f2; --surface: #ffffff; --surface-2: #eef1ea;
    --ink: #16211b; --muted: #5b6b60; --line: #dde3d9;
    --green: #1f8f5f; --green-soft: #e4f2ea; --copper: #c96a24;
    --danger: #d0432f; --warn: #c07a10;
    --mono: ui-monospace, "Cascadia Code", "SF Mono", Consolas, monospace;
    --sans: "Segoe UI", system-ui, -apple-system, Roboto, sans-serif;
  }
  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg: #0d1512; --surface: #14201a; --surface-2: #1a2820;
      --ink: #e7efe8; --muted: #90a498; --line: #26332b;
      --green: #3ad48c; --green-soft: #143026; --copper: #e2925a;
      --danger: #f0765f; --warn: #e0b155;
    }
  }
  :global(body) { margin: 0; background: var(--bg); color: var(--ink); font-family: var(--sans); }

  .center { min-height: 100vh; display: grid; place-items: center; padding: 24px; }
  .spinner { width: 34px; height: 34px; border: 3px solid var(--line); border-top-color: var(--green);
    border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }

  .gate, .wizard { max-width: 560px; background: var(--surface); border: 1px solid var(--line);
    border-radius: 18px; padding: 34px; box-shadow: 0 12px 40px rgba(0,0,0,.08); }
  .wizard { max-width: 620px; }
  h1 { font-size: 26px; letter-spacing: -.02em; margin: 14px 0 8px; text-wrap: balance; }
  .sub, .gate p { color: var(--muted); margin: 0 0 8px; }
  .fine { font-size: 13px; color: var(--muted); margin-top: 14px; }
  .hint { color: var(--muted); font-weight: 400; font-size: 12px; }

  .badge { display: inline-block; font-family: var(--mono); font-size: 12px; padding: 4px 10px;
    border-radius: 999px; font-weight: 600; }
  .badge.ok { background: var(--green-soft); color: var(--green); }
  .badge.warn { background: color-mix(in srgb, var(--warn) 16%, transparent); color: var(--warn); }

  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; margin: 20px 0; }
  label { display: flex; flex-direction: column; gap: 5px; font-size: 13px; font-weight: 600; }
  input, select { font: inherit; font-weight: 400; padding: 9px 11px; border-radius: 9px;
    border: 1px solid var(--line); background: var(--bg); color: var(--ink); }
  input:focus, select:focus { outline: 2px solid var(--green); outline-offset: 1px; border-color: transparent; }
  label.check { flex-direction: row; align-items: center; gap: 8px; font-weight: 400; margin-top: 6px; }
  label.check input { width: auto; }

  .mode { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin: 8px 0 22px; }
  .mode-opt { border: 1.5px solid var(--line); border-radius: 12px; padding: 14px; cursor: pointer;
    display: flex; flex-direction: column; align-items: flex-start; gap: 3px; text-align: left;
    width: 100%; font: inherit; color: inherit; background: transparent;
    transition: border-color .15s, background .15s; }
  .mode-opt span { font-size: 12.5px; color: var(--muted); }
  .mode-opt.active { border-color: var(--green); background: var(--green-soft); }

  button { font: inherit; font-weight: 600; padding: 9px 16px; border-radius: 10px; cursor: pointer;
    border: 1px solid var(--line); background: var(--surface); color: var(--ink); transition: filter .15s; }
  button:hover:not(:disabled) { filter: brightness(0.97); }
  button:disabled { opacity: .55; cursor: default; }
  button:focus-visible { outline: 2px solid var(--green); outline-offset: 2px; }
  button.primary { background: var(--green); color: #fff; border-color: transparent; }
  button.danger { color: var(--danger); border-color: color-mix(in srgb, var(--danger) 40%, var(--line)); }
  button.ghost { background: transparent; }
  button.big { width: 100%; padding: 13px; font-size: 15px; }
  .gate-actions { display: flex; gap: 10px; margin-top: 20px; }

  .toast { position: fixed; top: 18px; left: 50%; transform: translateX(-50%); z-index: 50;
    background: var(--ink); color: var(--bg); padding: 10px 18px; border-radius: 10px; font-size: 14px;
    box-shadow: 0 8px 24px rgba(0,0,0,.25); }

  .app { display: grid; grid-template-columns: 210px 1fr; min-height: 100vh; }
  aside { background: var(--surface); border-right: 1px solid var(--line); padding: 22px 16px;
    display: flex; flex-direction: column; gap: 22px; }
  .brand { font-size: 22px; font-weight: 800; letter-spacing: -.02em; }
  .brand span { color: var(--copper); }
  nav { display: flex; flex-direction: column; gap: 4px; }
  nav button { text-align: left; border: none; background: transparent; padding: 9px 12px; border-radius: 9px;
    color: var(--muted); font-weight: 600; }
  nav button.on { background: var(--green-soft); color: var(--green); }
  .state { margin-top: auto; font-size: 13px; font-weight: 600; display: flex; align-items: center; gap: 8px;
    flex-wrap: wrap; }
  .state small { width: 100%; color: var(--muted); font-weight: 400; font-family: var(--mono); }
  .dot { width: 9px; height: 9px; border-radius: 50%; }
  .dot.up { background: var(--green); box-shadow: 0 0 0 3px var(--green-soft); }
  .dot.starting { background: var(--warn); box-shadow: 0 0 0 3px color-mix(in srgb, var(--warn) 22%, transparent); }
  .dot.down { background: var(--muted); }

  main { padding: 30px 34px; max-width: 900px; }
  .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; margin-bottom: 22px; }
  .head h1 { margin: 0; font-size: 24px; }
  .controls { display: flex; gap: 8px; }

  .card { background: var(--surface); border: 1px solid var(--line); border-radius: 14px; padding: 20px;
    margin-bottom: 16px; }
  .card-title { font-weight: 700; margin-bottom: 12px; }
  .card.tunnel { border-color: color-mix(in srgb, var(--green) 30%, var(--line)); }
  .row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .row.end { justify-content: space-between; margin-top: 16px; }
  .row input { flex: 1; min-width: 160px; }
  .addr { display: flex; gap: 8px; align-items: center; background: var(--bg); border: 1px solid var(--line);
    border-radius: 10px; padding: 8px 8px 8px 14px; margin: 6px 0 12px; }
  .addr code { flex: 1; font-family: var(--mono); font-size: 15px; }
  .steps { margin: 6px 0 14px; padding-left: 18px; display: flex; flex-direction: column; gap: 10px;
    font-size: 14px; line-height: 1.5; }
  .steps code { font-family: var(--mono); font-size: 12.5px; background: var(--bg); border: 1px solid var(--line);
    padding: 1px 5px; border-radius: 5px; }
  .steps .row { margin-top: 6px; }
  button.link { background: transparent; border: none; color: var(--green); padding: 4px 0; font-weight: 600; }

  table { width: 100%; border-collapse: collapse; }
  th { text-align: left; font-size: 12px; text-transform: uppercase; letter-spacing: .06em; color: var(--muted);
    padding: 0 10px 10px; }
  td { padding: 12px 10px; border-top: 1px solid var(--line); }
  td.right { text-align: right; display: flex; gap: 6px; justify-content: flex-end; }
  .mono { font-family: var(--mono); font-size: 13px; color: var(--muted); }
  .empty { color: var(--muted); padding: 30px 0; text-align: center; }

  pre.logs { background: #0c1210; color: #cfe6d8; border-radius: 12px; padding: 16px; overflow: auto;
    max-height: 62vh; font-family: var(--mono); font-size: 12.5px; line-height: 1.6; white-space: pre-wrap; }

  @media (max-width: 760px) {
    .app { grid-template-columns: 1fr; }
    .grid, .mode { grid-template-columns: 1fr; }
    aside { flex-direction: row; align-items: center; }
    nav { flex-direction: row; }
    .state { margin: 0 0 0 auto; }
  }
</style>
