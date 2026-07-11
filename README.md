# PalBox

**One-click Palworld dedicated server — without exposing your IP.**

PalBox is a small desktop app that runs your own Palworld server for you and your
friends. No config files, no command line, no router setup. Friends connect
through a safe relay, so your home IP address is never shared.

> Palworld released today and everyone wants a server. Most guides assume you can
> edit `.env` files and forward ports. PalBox is for everyone else.

## What it does

- **Runs the server for you** — uses the proven
  [`palworld-server-docker`](https://github.com/thijsvanloef/palworld-server-docker)
  image under the hood, so the game server itself is battle-tested.
- **Keeps your IP private** — a built-in [playit.gg](https://playit.gg) tunnel
  gives your friends a relay address to join. No port-forwarding, and your public
  IP is never exposed. A "direct" mode is available if you prefer, with a clear
  warning.
- **Simple dashboard** — start/stop/restart, live player list, kick/ban,
  broadcast messages, backups, and settings, all from one window.

## Requirements

- **Windows 10/11**
- **[Docker Desktop](https://www.docker.com/products/docker-desktop/)** installed
  and running. PalBox checks for it and points you to the download if it's
  missing. This is the one prerequisite — everything else is automatic.
- ~8&nbsp;GB free disk (the server image is downloaded once) and enough RAM for
  Palworld (8&nbsp;GB is comfortable for a small group).

## Install

1. Download the latest `PalBox_x64-setup.exe` from
   [Releases](../../releases).
2. Run it. If Windows SmartScreen warns about an unknown publisher, click
   **More info → Run anyway** (the app isn't code-signed yet).
3. Open PalBox, follow the two-step setup, and click **Create my server**.

## How the "safe access" works

```
Friends ──▶ playit relay (UDP) ──▶ tunnel agent ──▶ Docker: Palworld server
            hides your IP          outbound only     isolated on your PC
```

The tunnel agent only makes an **outbound** connection to the relay, so nothing
listens on your router and your IP never appears anywhere. One-time setup: create
a free playit.gg account, add a **UDP tunnel** pointing at
`host.docker.internal:8211`, and paste the agent secret key into PalBox. After
that the relay address is ready to share, and your friends need nothing.

## Build from source

```bash
# prerequisites: Node 18+, Rust stable, and the Tauri v2 system deps
npm install
npm run tauri dev      # run in development
npm run tauri build    # produce the installer in src-tauri/target/release/bundle
```

## Project layout

```
src/               SvelteKit dashboard (the UI)
src-tauri/src/
  lib.rs           Tauri commands wiring the UI to the backend
  docker.rs        docker CLI wrapper (pull / run / stop / logs)
  tunnel.rs        playit.gg agent management
  config.rs        server settings -> container env vars
  rcon.rs          minimal Source RCON client for in-game admin
```

## Notes & limitations

- PalBox does **not** redistribute any Palworld game files. The server binaries
  are downloaded by Docker/SteamCMD on your own machine, from Valve.
- Tunnel reliability depends on playit.gg. If it doesn't suit you, "direct" mode
  (manual port-forward) is always available.
- macOS/Linux support is planned but not in this first release.

## License

MIT — see [LICENSE](LICENSE). Not affiliated with Pocketpair or Palworld.
