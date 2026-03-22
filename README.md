# GrayNet

**One-click launcher for the I2P anonymous network.**

GrayNet provides a simple way to access the I2P network with minimal setup. Install, click Connect, and the environment is configured automatically.

---

## What is I2P?

I2P (Invisible Internet Protocol) is an anonymous, encrypted network layer designed primarily for internal services such as websites, forums, and file sharing. Traffic is end-to-end encrypted and routed through multiple nodes.

## What is GrayNet?

GrayNet is a custom I2P ecosystem built on top of i2pd (the C++ I2P implementation). It adds:

- **`.gn` TLD (experimental)** — a custom top-level domain for GrayNet-native sites alongside standard `.i2p`
- **GrayNet Hub** — a central portal at `hub.gn` with links to network resources
- **One-click launcher** — handles i2pd daemon, browser setup, and proxy configuration automatically

---

## Features

- 🚀 **Minimal setup** — works out of the box in most cases
- 🌐 **Bundled i2pd** — no separate installation needed
- 🦊 **Auto-downloads LibreWolf** — privacy-focused browser, configured for I2P on first run (~85 MB)
- 🔒 **Pre-configured proxy** — `.i2p` and `.gn` traffic routed through i2pd, other traffic goes direct
- 📋 **Bundled address book** — popular `.i2p` sites available immediately
- 🔕 **Silent daemon** — i2pd runs in background without console window
- 📥 **System tray** — launcher minimizes to tray
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

> **Note:** On first launch, i2pd needs time to find peers and build tunnels. Some sites (e.g. `stats.i2p`) may be unavailable until the connection is established.

> **If something fails:** restart the app or check your internet connection.

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
