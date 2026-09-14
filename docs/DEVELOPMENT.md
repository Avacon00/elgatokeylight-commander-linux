# Development guide

[← Project overview](../README.md)

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
