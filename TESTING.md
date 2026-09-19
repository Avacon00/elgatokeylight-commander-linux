# Verification — 0.2.4

- Native COSMIC menu tests passed in German and English with isolated loopback fixtures. Brightness and temperature presets are direct children of the lamp menu, preventing COSMIC from collapsing the active lamp when a third-level submenu is opened.
- Individual 75% brightness and 3000 K actions reached only the selected fixture; offline controls, window-sync isolation, second-instance handling and clean Quit remain verified. Group writes were skipped because physical lights were discovered and remain covered by Rust simulations.
- 18 Rust tests and 6 frontend/translation tests passed. TypeScript, production frontend build, Rust formatting and Clippy with warnings denied passed.
- The production amd64 DEB built successfully. APT simulated upgrading the installed 0.2.3 to 0.2.4: one upgrade, no new packages or removals.
- The running installed application and physical lights were not modified. Native tests used a separate application identifier and configuration directory.

## Previous verification — 0.2.3

- 18 Rust tests and 6 frontend/translation tests passed. New cases inject write failures to verify OS autostart rollback, rollback-error reporting, device retention in memory/on disk and successful removal after retry.
- Browser integration passed, including a 113-character lamp name in the 400px detail window: Back, Scan and Settings remain inside the viewport. Screenshot inspected: `artifacts/details-long-name.png`.
- TypeScript, Clippy with warnings denied, formatting, production frontend and release DEB builds passed. Existing language, message and settings-isolation checks remain successful.
- APT simulated the installed 0.2.2 → 0.2.3 upgrade: one upgrade, no additional packages or removals.
- The running installed application and physical lamps were not modified. OS autostart changes in regression tests use an injected callback; UI integration uses a simulated Tauri bridge.

## Previous verification — 0.2.2

- TypeScript and production frontend/release DEB builds passed.
- Inspected the corrected closed language selector in native WebKitGTK 2.52 with the per-process GTK dark-theme preference disabled: selected text and arrow are clearly visible on the dark background. Screenshot: `artifacts/dropdown-webkit.png`. The open native popup was not separately inspected.
- Existing browser integration passed: immediate DE/EN/system switching, persisted selection, translated existing messages, isolated language payloads, unchanged checkbox settings and 400px layout.
- APT simulated the installed 0.2.1 → 0.2.2 upgrade: one upgrade, no new packages or removals. Installation and the running user instance were left untouched.
- `git diff --check` passed. This update changes selector styling and release metadata; prior backend verification below was not rerun.

## Previous verification — 0.2.1

Verified on Pop!_OS 24.04 / COSMIC / Wayland, amd64.

- **15 Rust tests passed**, including all prior lamp simulations plus locale selection/manual override, migration of 0.2.0 settings, language persistence, preservation of existing messages, isolated partial settings changes and rollback after a failed language save.
- **6 frontend/translation tests passed**: existing command queue scenarios, identical DE/EN catalog keys and placeholders, and catalog coverage of literal translation keys.
- **Browser integration passed** with a simulated Tauri bridge: immediate DE/EN/system switching, selection after reload, existing status/error translation, language-only command payloads, unchanged autostart/sync and 400px layout. German and English screenshots inspected. Slider names are exposed on the actual slider thumbs for assistive technology.
- **Native COSMIC menu tests passed in German and English** using a separate application identifier and loopback fixtures. Compact name/status rows, disabled submenu measurement rows, presets, individual targeting, Open/Settings, single instance and Quit verified. German offline test additionally verified disabled controls and absence of stale measurements.
- Clippy with warnings denied, TypeScript, Rust formatting and release DEB build passed.
- APT simulated upgrading the installed **0.2.0 → 0.2.1**: one upgrade, no additional packages or removals. Production identifier remains unchanged.

The running installed app and real light settings were not changed during these update tests. Native menu dispatch is tested through D-Bus; browser tests simulate the backend bridge. A live native window-to-backend language switch, Windows/macOS runtime behavior and a logout/login cycle have not been separately exercised. Installation of the new DEB is left to the user because sudo requires an interactive password.

## Previous verification — 0.2.0


Verified on Pop!_OS 24.04, COSMIC / Wayland, amd64.

## Completed

- `pnpm install --frozen-lockfile`, `pnpm lint`, `pnpm build`.
- Four frontend queue tests: rapid inputs, mixed properties, slow requests, error recovery and flushing the last value when leaving a view.
- Ten Rust tests with real loopback HTTP servers: field preservation, concurrent updates, partial failure, unconfirmed writes, identity/address changes, persistent settings, offline recovery, bounded timeouts, single-light targeting, identification restoration and graceful shutdown.
- `cargo clippy --all-targets -- -D warnings` and Rust formatting.
- Native COSMIC StatusNotifierItem/DBusMenu integration: icon registration, nested menus, individual on/off and presets, independence from window sync, Open/Settings actions, second-instance handling and clean Quit/unregister.
- Visual inspection of the actual native window on the desktop.
- Two physical Key Lights discovered automatically and their current states read successfully. Simulated lamps were used for control actions. Native group actions are skipped whenever physical lamps are discovered; group/partial-failure behavior is covered by the Rust tests.
- Release amd64 DEB generated with application binary, desktop launcher, icons and dependency metadata.

The loopback tests and desktop integration require permission to open local sockets/access the session bus. Restricted execution without that access fails before exercising application logic; the permitted runs passed.

## Not yet verified

- Intentional changes to the two physical lamps, including physical identification blinking.
- An actual logout/login cycle for autostart.
- Physical pointer interaction with the panel menu; native menu dispatch was exercised through its D-Bus interface.
- Runtime fallback on a desktop without a tray host, panel restart behavior, and live Windows/macOS behavior. A cross-platform CI build/test matrix is provided but was not run from this local fork.
- System-wide package installation: this host requires an interactive sudo password. The package itself can be installed using the command in README.md.
