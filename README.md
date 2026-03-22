# GrayNet

**One-click launcher for the I2P anonymous network.**

GrayNet bundles everything you need to access the I2P network — no manual configuration, no technical knowledge required. Install, click Connect, and you're in.

---

## What is I2P?

I2P (Invisible Internet Protocol) is an anonymous, encrypted network layer. Unlike Tor which focuses on accessing the clearweb anonymously, I2P is designed for internal services — websites, forums, file sharing — that exist entirely within the network. Traffic is end-to-end encrypted and routed through multiple nodes, making it difficult to trace.

## What is GrayNet?

GrayNet is a custom I2P ecosystem built on top of i2pd (the C++ I2P implementation). It adds:

- **`.gn` TLD** — a custom top-level domain for GrayNet-native sites alongside standard `.i2p`
- **GrayNet Hub** — a central portal at `hub.gn` with links to network resources
- **One-click launcher** — handles i2pd daemon, browser setup, and proxy configuration automatically

---

## Features

- 🚀 **Zero configuration** — works out of the box
- 🌐 **Bundled i2pd** — no separate installation needed
- 🦊 **Auto-downloads LibreWolf** — privacy-focused browser, configured for I2P on first run
- 🔒 **Pre-configured proxy** — PAC file routes `.i2p` and `.gn` traffic through i2p, everything else goes direct
- 📋 **Bundled address book** — popular `.i2p` sites available immediately, no waiting for subscriptions
- 🔕 **Silent daemon** — i2pd runs in background with no tray icon or console window
- 📥 **System tray** — launcher minimizes to tray, i2p keeps running
- 🔄 **Single instance** — only one launcher can run at a time

---

## Installation

1. Download `GrayNet_x64-setup.exe` from the [releases page](https://github.com/vialolis/GrayNet/releases)
2. Run the installer
3. Launch GrayNet from the desktop shortcut
4. Click **Install Browser** on first run — LibreWolf will download automatically (~85 MB)
5. Click **Start I2P**
6. Wait 2–5 minutes for the daemon to integrate into the network
7. Click **Open I2P Browser**

> **Note:** The first time you connect, i2pd needs to find peers and build tunnels. This can take a few minutes. Sites like `stats.i2p` will be unreachable until tunnels are established.

---

## GrayNet Sites

| Site | Address | Description |
|------|---------|-------------|
| GrayNet Hub | `hub.gn` | Central portal |
| GrayNet Forum | `forum.hub.gn` | Community forum |
| GrayNet on I2P | `graynet.i2p` | Main site |
| Stats | `stats.i2p` | I2P network statistics |

---

## Architecture

```
GrayNet Launcher (Tauri + Rust)
├── i2pd.exe          — I2P daemon, runs silently in background
├── LibreWolf         — Pre-configured browser (downloaded on first run)
│   ├── proxy.pac     — Routes .i2p and .gn through i2pd HTTP proxy (port 4444)
│   └── prefs.js      — Disables WebRTC, telemetry, DNS-over-HTTPS
└── addresses.csv     — Pre-seeded address book with popular .i2p sites
```

### Custom `.gn` TLD

GrayNet uses a patched version of i2pd that recognizes `.gn` domains in addition to standard `.i2p`. Resolution is handled through a `zones.txt` file — a simple `hostname=b32address` mapping loaded at daemon startup.

---

## Building from Source

### Prerequisites

- Rust (stable) — [rustup.rs](https://rustup.rs)
- Tauri CLI v1 — `cargo install tauri-cli --version "^1.6" --locked`
- Node.js (optional, for Tauri CLI via npm)
- Windows: Visual Studio Build Tools or MSVC

### Build

```bash
git clone https://github.com/vialolis/GrayNet
cd GrayNet
cargo tauri build
```

Installer will be at `target/release/bundle/nsis/GrayNet_x64-setup.exe`

### Development

```bash
cargo build        # debug build
cargo tauri dev    # dev mode with hot reload (if using frontend bundler)
```

### i2pd with GrayNet patches

The patched i2pd with `.gn` TLD support is at [vialolis/i2pd_graynet](https://github.com/vialolis/i2pd_graynet). Build it with Visual Studio using the `openssl` branch, then place `i2pd.exe` in the `binaries/` folder before running `cargo tauri build`.

---

## Configuration

GrayNet stores all data in `%APPDATA%\GrayNet\`:

```
GrayNet\
  bin\i2pd.exe          — i2pd binary
  browser\              — LibreWolf portable
  config\i2pd.conf      — i2pd configuration
  config\addresses.csv  — address book
  i2pd\                 — i2pd runtime data (router keys, netDb, etc.)
  logs\i2pd.log         — i2pd logs
  proxy.pac             — proxy auto-config
```

### i2pd config

Default config is minimal. You can customize `%APPDATA%\GrayNet\config\i2pd.conf`:

```ini
[addressbook]
subscriptions=http://graynet.i2p/hosts.txt,http://reg.i2p/hosts.txt
```

---

## Privacy Notes

- All `.i2p` and `.gn` traffic is routed through i2pd — your real IP is not exposed to I2P sites
- Clearweb traffic goes direct (not through I2P) — this is intentional for performance
- LibreWolf has WebRTC disabled to prevent IP leaks
- No telemetry is collected by GrayNet or the bundled browser config
- JavaScript is enabled — required for modern I2P sites like forums

---

## License

GrayNet launcher — BSD 3-Clause  
i2pd — BSD 3-Clause ([PurpleI2P/i2pd](https://github.com/PurpleI2P/i2pd))  
LibreWolf — MPL-2.0 ([librewolf.net](https://librewolf.net))
