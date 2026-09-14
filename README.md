# Keylight Commander Linux

A Linux-first derivative of [David Chalifoux's Keylight Commander](https://github.com/davidchalifoux/keylight-commander), based on `5aec9305b6dc3a989a81c66ebc2692070462e426` (v0.1.1). Original copyright and MIT license are preserved in [license.md](license.md).

Control Elgato Key Lights from a native panel menu on Pop!_OS 24.04 / COSMIC / Wayland. Tauri and the existing React controls remain; the device controller now lives in Rust and works independently of the window. The upstream auto-updater is disabled so original releases cannot replace this derivative.

## Repository and license

This project retains the upstream Git history and MIT license. It is maintained as an independent repository so it can remain private. The original project is available at [davidchalifoux/keylight-commander](https://github.com/davidchalifoux/keylight-commander); the local `upstream` remote tracks that source.

See [TESTING.md](TESTING.md) for verification results and known limits. Build tools, device configurations and generated packages are excluded from Git. The GitHub Actions workflow supports manual builds and pull requests; it does not publish releases.

## Version 0.2.3: layout and save-error fixes

Long device names are truncated in the detail header so navigation remains accessible. Failed settings saves restore the previous OS autostart preference; rollback failures are reported and the displayed value follows the last successful OS change. Failed device removals retain the device in memory and on disk. Persistence operations serialize these changes to avoid saving an intermediate state.

## Version 0.2.2: readable language dropdown

The language selector now uses explicit dark colors and its own arrow, avoiding the light native GTK background that made its text unreadable. Keyboard selection and focus remain available.

## Version 0.2.1: language and compact menus

Select **System language / Systemsprache**, **Deutsch**, or **English** under **Settings / Einstellungen → Sprache / Language**. The window and panel update immediately, including messages already on screen. System mode chooses German for German locales and English otherwise; manual choices are saved. Configurations from 0.2.0 automatically use system mode, preserving lights, autostart and synchronization.

Panel rows now show only the original device name and On/Off (An/Aus). Current brightness and color temperature appear as a disabled status row inside each device submenu. Offline devices do not display stale measurements and their controls remain disabled.

To update an earlier installed version, quit the running app from the panel, install the 0.2.3 DEB using your package manager, and launch it again. No device setup is required after the update.

## Everyday use

Install the generated amd64 DEB with your package manager, then launch **Keylight Commander Linux**. The first launch opens the window, scans the local network and enables autostart. Lights must already be connected to your network; this app does not provision their Wi-Fi.

Click the lamp icon in COSMIC's **Status Area** to open the menu:

- Turn all reachable lights on/off, or select one light.
- Brightness presets: 10, 25, 50, 75 and 100 percent.
- Color temperature presets: 3000, 4000, 5000 and 6500 K.
- Open the window for continuous sliders, names, device details and identification blinking.
- Scan adds or refreshes devices without deleting offline ones. Remove explicitly forgets a device; a later scan can discover it again.

Individual panel entries always control only that light. **Sync window controls** affects window controls only. Offline lamps remain listed and are disabled in the menu. Failed or partially successful commands appear in the window and panel menu.

Closing the window keeps the panel active. **Quit** waits for active operations (including restoration after identification) and exits. Starting the app again opens the existing instance. If no tray host is available, the window stays accessible and closing it exits. Disable **Start automatically in the panel** in Settings to stop launching on login.

This fork has its own application identifier and configuration. It discovers lamps afresh; it does not import or overwrite the upstream application's browser storage. Linux configuration is stored at `$XDG_CONFIG_HOME/io.github.keylight-commander-linux/settings.json` (normally `~/.config/io.github.keylight-commander-linux/settings.json`). The autostart plugin creates `~/.config/autostart/Keylight Commander Linux.desktop`.

## Build and test

Requirements: Node.js 22+, pnpm 11, stable Rust, and Linux development packages:

```sh
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libssl-dev
pnpm install --frozen-lockfile
pnpm test
pnpm build
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pnpm tauri build --bundles deb
```

The DEB is written to `src-tauri/target/release/bundle/deb/`. Install it with `sudo apt install ./path/to/package.deb`. Debian dependencies target Pop!_OS / Ubuntu 24.04; other distributions may require their own packaging.

On the original development machine a rootless toolchain is available under `.build-tools/`. Run `source scripts/local-env.sh` before building there. These downloaded tools and packages are intentionally untracked; regular installations can use the system dependencies above.

The CI workflow builds and runs tests on Linux, macOS and Windows without publishing. Only Linux creates a DEB. Local cross-platform builds are not evidence of macOS/Windows runtime behavior; native verification is still needed there.

### Native COSMIC integration test

With a running desktop session and Python Gio bindings:

```sh
/usr/bin/python3 tests/native-tray.py "$PWD/src-tauri/target/release/keylight-commander-linux"
```

The test runs two simulated loopback lamps with isolated application configuration and exercises the actual StatusNotifierItem/DBusMenu interface. Group actions are skipped if physical devices are discovered. It does not change your autostart setting. This proves menu dispatch and device behavior; visual interaction and physical lamps require additional checks.

## Internal behavior

The Rust controller owns discovery, saved devices, settings and confirmed states. Internal Tauri commands are `snapshot`, `scan`, `change_light`, `rename_light`, `remove_light`, `identify_light`, and `update_settings` (optional `autostart`, `sync`, and `language` changes); `lights-changed` broadcasts snapshots and `navigate` selects the window page. No HTTP server is added to the application.

mDNS supplies the service address and port. Serial number, falling back to MAC address, identifies devices across IP changes. A shared HTTP client has a two-second connection timeout and four-second total request timeout. Per-device locks serialize polling and writes; read/modify/write preserves other properties and successful writes are read back before confirmation. Frontend queues merge rapid changes without accumulating outdated slider requests.

Visible windows refresh approximately every two seconds; background operation refreshes approximately every ten seconds. Offline lamps retry after at least thirty seconds. Slow requests can extend these intervals. Lamp states are not persisted as current truth across restarts. Communication stays on the local network using the lamps' existing HTTP API.

Translations are shared JSON catalogs under `src/locales/`. Snapshots include the effective `locale`; saved settings contain `language` (`system`, `de`, or `en`). Application messages use keys, parameters and nested causes so they can be rendered in either language without changing the underlying operation. Third-party diagnostic details remain verbatim.

The optional browser check in `tests/frontend-ui.mjs` uses a simulated Tauri bridge and an installed Playwright module (`PLAYWRIGHT_MODULE` can point to it). It verifies live language changes and 400px layouts. Native menu tests accept `--language=de` or `--language=en`. To avoid interfering with an already running installation, build a test copy with `--config '{"identifier":"io.github.keylight-commander-linux.test"}'` and pass `--identifier=io.github.keylight-commander-linux.test` to the native test. Build the production DEB afterwards with the normal configuration.
