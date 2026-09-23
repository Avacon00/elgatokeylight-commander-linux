<div align="center">
  <img src="app-icon.png" alt="Keylight Commander Linux icon" width="112" />
  <h1>Keylight Commander Linux</h1>
  <p><strong>Control Elgato Key Lights from the Linux system tray.</strong></p>
  <p>Local, fast and cloud-free controls for Ubuntu, Pop!_OS, COSMIC and Wayland desktops.</p>
  <p>
    <a href="https://github.com/Avacon00/elgatokeylight-commander-linux/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/Avacon00/elgatokeylight-commander-linux?display_name=tag&style=flat-square"></a>
    <a href="https://github.com/Avacon00/elgatokeylight-commander-linux/actions/workflows/test-on-pr.yml"><img alt="Build and test" src="https://img.shields.io/github/actions/workflow/status/Avacon00/elgatokeylight-commander-linux/test-on-pr.yml?style=flat-square&label=build"></a>
    <a href="license.md"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-blue?style=flat-square"></a>
    <img alt="Linux" src="https://img.shields.io/badge/platform-Linux-FCC624?style=flat-square&logo=linux&logoColor=black">
  </p>
  <p>
    <a href="#installation">Installation</a> ·
    <a href="#features">Features</a> ·
    <a href="#tested-platforms">Compatibility</a> ·
    <a href="README.de.md">Deutsch</a> ·
    <a href="CONTRIBUTING.md">Contributing</a>
  </p>
</div>

---

**Keylight Commander Linux** is a desktop and tray application for controlling one or more Elgato Key Lights over the local network. It discovers lights through mDNS and controls them directly through their local HTTP API. No cloud account or external server is involved.

The app is optimized and manually tested on **Pop!_OS 24.04 with COSMIC on Wayland**. It is built and automatically tested on **Ubuntu 24.04**. Other Linux distributions and desktop environments have not yet been verified; test reports are very welcome.

## Screenshot

<p align="center">
  <img src="readme-images/linux-tray-0.2.4.png" alt="Keylight Commander Linux tray menu on Pop!_OS COSMIC" width="390" />
</p>

The native panel menu provides quick controls. The desktop window remains available for precise brightness and color-temperature sliders.

## Installation

Download the latest **amd64 DEB** from [GitHub Releases](https://github.com/Avacon00/elgatokeylight-commander-linux/releases/latest), then install it with:

```sh
sudo apt install ./keylight-commander-linux_0.2.4_amd64.deb
```

Launch **Keylight Commander Linux** from the application menu. On first launch, the app opens its control window, searches the local network and enables login autostart. Your Key Lights must already be connected to the same network; the app does not configure their Wi-Fi.

To update, quit the running app from its tray menu before installing the new DEB. Saved lights and settings are retained.

## Features

### Native Linux tray controls

- Persistent lamp icon in the COSMIC status area through AppIndicator/StatusNotifierItem.
- Turn all reachable lights on or off.
- Control one selected light without affecting the others.
- Brightness presets: **10, 25, 50, 75 and 100%**.
- Color-temperature presets: **3000, 4000, 5000 and 6500 K**.
- Current power, brightness and color temperature shown per online light.
- Offline lights remain visible with disabled controls and no stale readings.
- Search for lights, open the control window, open settings or quit from the panel.

### Precise desktop controls

- Continuous brightness control from **3 to 100%**.
- Continuous color-temperature control from approximately **2900 to 7000 K**.
- Optional synchronization of window controls across all reachable lights.
- Rename lights and inspect model, serial number, firmware, MAC address and endpoint.
- Identification blink that restores the previous confirmed light state.

### Discovery and reliability

- Automatic local discovery through mDNS; host and port are taken from the service record.
- Devices are recognized by serial number, with MAC address as fallback, so changed IP addresses are handled.
- New scans merge updated devices without deleting saved offline lights.
- Per-device command serialization and coalescing of rapid slider changes.
- HTTP status checks, bounded timeouts and immediate readback after writes.
- Group actions report partial failures instead of displaying unconfirmed success.
- Visible windows refresh approximately every 2 seconds; background state refreshes approximately every 10 seconds.

### Desktop integration

- Runs in the background when the window is closed.
- Optional autostart on login, configurable in Settings.
- Starting the app again opens the existing instance.
- Falls back to a visible window when no compatible tray host is available.
- Separate application identity and configuration from the original upstream app.
- German and English interface with automatic system-language selection and instant switching.

## Tested platforms

| Platform | Status |
| :--- | :--- |
| **Pop!_OS 24.04 · COSMIC · Wayland · amd64** | Native tray, window, background mode and DEB tested locally |
| **Ubuntu 24.04 · amd64** | Build, automated tests and DEB packaging covered |
| Other Linux distributions/desktops | Not tested yet; AppIndicator and WebKitGTK compatibility may vary |
| Windows / macOS | Source compatibility is retained, but runtime behavior is not verified |

If you test another Linux distribution, desktop environment or Key Light model, please [open an issue](https://github.com/Avacon00/elgatokeylight-commander-linux/issues/new/choose). Include the distribution, desktop/session, app version and light model. Bug reports in **German or English** are welcome.

## Configuration

| Data | Default Linux location |
| :--- | :--- |
| Saved lights and settings | `~/.config/io.github.keylight-commander-linux/settings.json` |
| Login autostart entry | `~/.config/autostart/Keylight Commander Linux.desktop` |

The application communicates only with lights on the local network. Confirmed live states are not persisted as current truth across restarts.

## Development and testing

See the [development guide](docs/DEVELOPMENT.md) for dependencies, build commands, architecture and the native COSMIC integration test.

Version 0.2.4 passes **18 Rust tests**, **6 frontend/translation tests**, Clippy with warnings denied, TypeScript, production builds and native German/English COSMIC menu tests using isolated simulated lights. Detailed results and remaining hardware-dependent limits are documented in [TESTING.md](TESTING.md).

Contributions and reproducible bug reports are welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md), search existing issues, and open a new report when needed.

## Project history and license

This project is an independent Linux-focused derivative of [David Chalifoux's Keylight Commander](https://github.com/davidchalifoux/keylight-commander), based on upstream commit [`5aec930`](https://github.com/davidchalifoux/keylight-commander/commit/5aec9305b6dc3a989a81c66ebc2692070462e426) (v0.1.1).

The original Git history, copyright notices and [MIT license](license.md) are preserved. The upstream auto-updater is disabled so releases from the original application cannot replace this Linux build.
