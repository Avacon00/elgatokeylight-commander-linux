<div align="center">
  <img src="app-icon.png" alt="Keylight Commander Linux icon" width="112" />
  <h1>Keylight Commander Linux</h1>
  <p><strong>Your lights. One panel menu.</strong></p>
  <p>Local control for Elgato Key Lights, built around the Linux desktop.</p>
  <p><strong>v0.2.3</strong> · Pop!_OS / COSMIC · DE / EN · MIT</p>
  <p>
    <a href="#get-started">Get started</a> ·
    <a href="#panel-controls">Panel controls</a> ·
    <a href="docs/DEVELOPMENT.md">Build & contribute</a> ·
    <a href="TESTING.md">Test results</a>
  </p>
</div>

---

Control one light or your whole setup from the status area. Keep precise sliders in the window, then close it and let the panel handle the everyday adjustments.

Designed for **Pop!_OS 24.04 with COSMIC / Wayland**, with a native Linux menu and a shared Rust controller behind both interfaces.

## At a glance

| Feature | What you get |
| :--- | :--- |
| **Native panel menu** | Power, brightness and temperature without opening a window |
| **Multiple lights** | Individual controls and clearly separated group actions |
| **Precise adjustments** | Window sliders, device details, renaming and identification blinking |
| **Deutsch & English** | Instant language switching, or automatic system-language selection |
| **Background operation** | Optional login autostart and a single running instance |
| **Local communication** | mDNS discovery and HTTP control on your network |
| **Confirmed states** | Readback after commands, offline indicators and partial-failure reporting |

## Get started

### 1. Install the Linux package

Build an **amd64 DEB** using the [development guide](docs/DEVELOPMENT.md), or obtain a package from a completed manual [GitHub Actions build](https://github.com/Avacon00/elgatokeylight-commander-linux/actions/workflows/test-on-pr.yml). Generated packages are not stored in this repository; there is no published release download yet.

```sh
sudo apt install "./Keylight Commander Linux_0.2.3_amd64.deb"
```

Use the actual filename if your package was renamed. The package targets Pop!_OS / Ubuntu 24.04.

### 2. Connect your lights

Make sure your Key Lights are already connected to your network, then launch **Keylight Commander Linux**. The first launch opens the window, searches for lights and enables login autostart. The app does not provision Wi-Fi.

### 3. Use the panel

Enable COSMIC’s **Status Area**, then click the lamp icon. Open **Settings → Sprache / Language** to choose **System language**, **Deutsch** or **English**. System mode uses German for German locales and English otherwise.

> **Updating an existing installation?** Quit the app from its panel menu, install the new DEB and launch it again. Your saved lights and settings are retained.

## Panel controls

| Action | Available controls |
| :--- | :--- |
| **All reachable lights** | On / off, brightness and temperature presets |
| **One light** | Dedicated submenu with its current state and controls |
| **Brightness** | 10 · 25 · 50 · 75 · 100 % |
| **Color temperature** | 3000 · 4000 · 5000 · 6500 K |
| **More** | Scan for lights, open controls, settings and quit |

Lamp rows stay compact: **Name · On / Off / Offline**. Current brightness and temperature appear inside the lamp submenu. Offline lamps remain listed, with disabled controls and no stale measurements shown as current.

**Individual panel actions control only the selected lamp.** The optional **Sync window controls** setting applies to the window sliders and controls.

## Fits your desktop

- **Close the window:** the panel keeps running.
- **Launch again:** the existing window opens instead of starting another instance.
- **Quit from the panel:** active operations finish, including restoring a light after identification.
- **Disable login autostart:** turn off **Start automatically in the panel** in Settings.
- **No tray host available:** the window remains accessible; closing it exits the app.

Scanning refreshes addresses and adds new lamps without deleting offline ones. **Remove** forgets a saved lamp; a later scan can discover it again.

<details>
<summary><strong>Configuration and local data</strong></summary>

The application uses its own identity, separate from the upstream app. It discovers devices afresh and does not import or overwrite upstream browser storage.

| Data | Linux location |
| :--- | :--- |
| Settings and saved devices | `$XDG_CONFIG_HOME/io.github.keylight-commander-linux/settings.json` |
| Default configuration location | `~/.config/io.github.keylight-commander-linux/settings.json` |
| Login autostart entry | `~/.config/autostart/Keylight Commander Linux.desktop` |

Device configurations, local build tools and generated packages are excluded from Git. The original application's auto-updater is disabled so upstream releases cannot replace this version.

</details>

## What’s new

| Version | Changes |
| :--- | :--- |
| **0.2.3** | Long names keep navigation accessible; failed saves restore autostart changes and preserve removed devices |
| **0.2.2** | Readable dark language selector under native GTK themes |
| **0.2.1** | Shared German / English translations, saved language preference and compact panel menus |
| **0.2.0** | Linux panel controls and shared Rust device management |

## Development & verification

**18 Rust tests + 6 frontend / translation tests passed for 0.2.3**, alongside browser integration, Clippy and the release DEB build. These are recorded local results, not a live CI status badge.

- [Build, test and architecture guide](docs/DEVELOPMENT.md)
- [Verification results and known limitations](TESTING.md)
- [GitHub Actions workflow](.github/workflows/test-on-pr.yml)

The workflow supports pull requests and manual runs across Linux, macOS and Windows; only Linux produces a DEB. Windows/macOS runtime behavior and all physical-device scenarios have not been verified. See the test report for the exact scope.

## Credits & license

Based on [David Chalifoux’s Keylight Commander](https://github.com/davidchalifoux/keylight-commander), starting from upstream commit [`5aec930`](https://github.com/davidchalifoux/keylight-commander/commit/5aec9305b6dc3a989a81c66ebc2692070462e426) (v0.1.1).

The original Git history, copyright notices and [MIT license](license.md) are preserved. This derivative is maintained as an independent repository with its own application identity.
