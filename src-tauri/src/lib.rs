mod i18n;
mod lights;
use i18n::{text, Language, Locale, Message};
use lights::{Controller, Outcome, Patch, Snapshot};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    Emitter, Manager, State,
};
use tauri_plugin_autostart::ManagerExt;

type MenuActions = std::collections::HashMap<String, (Option<String>, Patch)>;

#[derive(Clone)]
struct Runtime {
    controller: Controller,
    tray_available: Arc<AtomicBool>,
    menu_state: Arc<tokio::sync::Mutex<String>>,
    actions: Arc<tokio::sync::RwLock<MenuActions>>,
}
fn quit(app: &tauri::AppHandle) {
    let app = app.clone();
    let runtime = app.state::<Runtime>().inner().clone();
    tauri::async_runtime::spawn(async move {
        *runtime.controller.message.write().await = Some(Message::new("quitting"));
        publish(&app, &runtime).await;
        runtime.controller.finish_operations().await;
        app.exit(0);
    });
}
fn show(app: &tauri::AppHandle, settings: bool) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    let _ = app.emit("navigate", if settings { "settings" } else { "lights" });
}
#[cfg(target_os = "linux")]
fn has_tray_host() -> bool {
    // Name ownership works on Wayland and does not depend on tray mouse events.
    [
        "org.kde.StatusNotifierWatcher",
        "org.freedesktop.StatusNotifierWatcher",
    ]
    .iter()
    .any(|name| {
        std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--timeout",
                "2",
                "--dest",
                "org.freedesktop.DBus",
                "--object-path",
                "/org/freedesktop/DBus",
                "--method",
                "org.freedesktop.DBus.NameHasOwner",
                name,
            ])
            .output()
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("true"))
            .unwrap_or(false)
    })
}
#[cfg(not(target_os = "linux"))]
fn has_tray_host() -> bool {
    true
}

fn presets(
    app: &tauri::AppHandle,
    menu: &Submenu<tauri::Wry>,
    prefix: &str,
    id: Option<String>,
    enabled: bool,
    locale: Locale,
    actions: &mut std::collections::HashMap<String, (Option<String>, Patch)>,
) -> tauri::Result<()> {
    for (suffix, title, patch) in [
        (
            "on",
            text(locale, "turn_on"),
            Patch {
                on: Some(1),
                ..Default::default()
            },
        ),
        (
            "off",
            text(locale, "turn_off"),
            Patch {
                on: Some(0),
                ..Default::default()
            },
        ),
    ] {
        let key = format!("{prefix}-{suffix}");
        menu.append(&MenuItem::with_id(app, &key, title, enabled, None::<&str>)?)?;
        actions.insert(key, (id.clone(), patch));
    }
    let brightness = Submenu::new(app, text(locale, "brightness"), enabled)?;
    for value in [10, 25, 50, 75, 100] {
        let key = format!("{prefix}-b{value}");
        brightness.append(&MenuItem::with_id(
            app,
            &key,
            format!("{value}%"),
            enabled,
            None::<&str>,
        )?)?;
        actions.insert(
            key,
            (
                id.clone(),
                Patch {
                    brightness: Some(value),
                    ..Default::default()
                },
            ),
        );
    }
    let temperature = Submenu::new(app, text(locale, "temperature"), enabled)?;
    for kelvin in [3000, 4000, 5000, 6500] {
        let key = format!("{prefix}-t{kelvin}");
        temperature.append(&MenuItem::with_id(
            app,
            &key,
            format!("{kelvin} K"),
            enabled,
            None::<&str>,
        )?)?;
        actions.insert(
            key,
            (
                id.clone(),
                Patch {
                    temperature: Some((1_000_000.0 / kelvin as f64).round() as u16),
                    ..Default::default()
                },
            ),
        );
    }
    menu.append(&brightness)?;
    menu.append(&temperature)?;
    Ok(())
}
async fn publish(app: &tauri::AppHandle, runtime: &Runtime) {
    // Serialize snapshots as well as menu replacement: an older poll must not
    // overwrite a newly selected language.
    let mut previous = runtime.menu_state.lock().await;
    let snapshot = runtime.controller.snapshot().await;
    let locale = snapshot.locale;
    let _ = app.emit("lights-changed", &snapshot);
    let signature = serde_json::to_string(&snapshot).unwrap_or_default();
    if *previous == signature {
        return;
    }
    let result = (|| -> tauri::Result<_> {
        let menu = Menu::new(app)?;
        let mut actions = std::collections::HashMap::new();
        let any = snapshot
            .devices
            .iter()
            .any(|d| d.state.is_some() && d.error.is_none());
        menu.append(&MenuItem::with_id(
            app,
            "all-on",
            text(locale, "all_on"),
            any,
            None::<&str>,
        )?)?;
        menu.append(&MenuItem::with_id(
            app,
            "all-off",
            text(locale, "all_off"),
            any,
            None::<&str>,
        )?)?;
        actions.insert(
            "all-on".into(),
            (
                None,
                Patch {
                    on: Some(1),
                    ..Default::default()
                },
            ),
        );
        actions.insert(
            "all-off".into(),
            (
                None,
                Patch {
                    on: Some(0),
                    ..Default::default()
                },
            ),
        );
        let all = Submenu::new(app, text(locale, "all_lights"), any)?;
        presets(app, &all, "all", None, any, locale, &mut actions)?;
        menu.append(&all)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        if snapshot.devices.is_empty() {
            menu.append(&MenuItem::with_id(
                app,
                "empty",
                text(locale, "empty_menu"),
                false,
                None::<&str>,
            )?)?;
        }
        for view in &snapshot.devices {
            let key = view
                .device
                .id
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let online = view.error.is_none() && view.state.is_some();
            let status = if online {
                text(
                    locale,
                    if view.state.as_ref().unwrap().on == 1 {
                        "on"
                    } else {
                        "off"
                    },
                )
            } else {
                text(locale, "offline")
            };
            let sub = Submenu::new(app, format!("{} · {status}", view.device.name), true)?;
            if online {
                let state = view.state.as_ref().unwrap();
                let label = Message::new("light_status")
                    .param("brightness", state.brightness)
                    .param(
                        "kelvin",
                        (1_000_000.0 / state.temperature as f64).round() as u32,
                    )
                    .render(locale);
                sub.append(&MenuItem::with_id(
                    app,
                    format!("status-{key}"),
                    label,
                    false,
                    None::<&str>,
                )?)?;
                sub.append(&PredefinedMenuItem::separator(app)?)?;
            }
            presets(
                app,
                &sub,
                &format!("device-{key}"),
                Some(view.device.id.clone()),
                online,
                locale,
                &mut actions,
            )?;
            menu.append(&sub)?;
        }
        if let Some(message) = &snapshot.message {
            menu.append(&PredefinedMenuItem::separator(app)?)?;
            menu.append(&MenuItem::with_id(
                app,
                "error",
                Message::new("attention")
                    .param(
                        "message",
                        message.render(locale).chars().take(100).collect::<String>(),
                    )
                    .render(locale),
                false,
                None::<&str>,
            )?)?;
        }
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        for (id, title, enabled) in [
            (
                "scan",
                if snapshot.scanning {
                    text(locale, "scanning")
                } else {
                    text(locale, "scan")
                },
                !snapshot.scanning,
            ),
            ("open", text(locale, "open_controls"), true),
            ("settings", text(locale, "settings"), true),
            ("quit", text(locale, "quit"), true),
        ] {
            menu.append(&MenuItem::with_id(app, id, title, enabled, None::<&str>)?)?;
        }
        Ok((menu, actions))
    })();
    match result {
        Ok((menu, actions)) => {
            if let Some(tray) = app.tray_by_id("lights") {
                match tray.set_menu(Some(menu)) {
                    Ok(()) => {
                        *runtime.actions.write().await = actions;
                        *previous = signature;
                    }
                    Err(e) => {
                        eprintln!("Tray menu: {e}");
                        runtime.tray_available.store(false, Ordering::Relaxed);
                        show(app, false);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Tray menu: {e}");
            runtime.tray_available.store(false, Ordering::Relaxed);
            show(app, false);
        }
    }
}
#[tauri::command]
async fn snapshot(runtime: State<'_, Runtime>) -> Result<Snapshot, Message> {
    Ok(runtime.controller.snapshot().await)
}
#[tauri::command]
async fn scan(app: tauri::AppHandle, runtime: State<'_, Runtime>) -> Result<(), Message> {
    let controller = runtime.controller.clone();
    let app2 = app.clone();
    let runtime2 = runtime.inner().clone();
    // Publish scanning state while discovery is in progress.
    let task = tauri::async_runtime::spawn(async move { controller.discover().await });
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    publish(&app2, &runtime2).await;
    let result = task.await.map_err(|e| e.to_string())?;
    publish(&app, &runtime).await;
    result
}
#[tauri::command]
async fn change_light(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
    id: Option<String>,
    patch: Patch,
    sync: bool,
) -> Result<Outcome, Message> {
    patch.apply(&mut lights::LightState::default())?;
    let targets = runtime.controller.targets(id, sync).await;
    let result = runtime.controller.change(targets, patch).await;
    publish(&app, &runtime).await;
    Ok(result)
}
#[tauri::command]
async fn rename_light(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
    id: String,
    name: String,
) -> Result<(), Message> {
    let result = runtime.controller.rename(&id, name).await;
    publish(&app, &runtime).await;
    result
}
#[tauri::command]
async fn remove_light(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
    id: String,
) -> Result<(), Message> {
    runtime.controller.remove(&id).await?;
    publish(&app, &runtime).await;
    Ok(())
}
#[tauri::command]
async fn identify_light(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
    id: String,
) -> Result<(), Message> {
    let result = runtime.controller.identify(&id).await;
    if let Err(e) = &result {
        *runtime.controller.message.write().await = Some(e.clone());
    }
    publish(&app, &runtime).await;
    result
}
#[tauri::command]
async fn update_settings(
    app: tauri::AppHandle,
    runtime: State<'_, Runtime>,
    autostart: Option<bool>,
    sync: Option<bool>,
    language: Option<Language>,
) -> Result<(), Message> {
    let result = runtime
        .controller
        .update_settings(autostart, sync, language, |enabled| {
            if enabled {
                app.autolaunch().enable()
            } else {
                app.autolaunch().disable()
            }
            .map_err(|e| Message::new("error.autostart").cause(e.to_string()))
        })
        .await;
    publish(&app, &runtime).await;
    result
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show(app, false)
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            snapshot,
            scan,
            change_light,
            rename_light,
            remove_light,
            identify_light,
            update_settings
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if let Some(runtime) = window.app_handle().try_state::<Runtime>() {
                    if runtime.tray_available.load(Ordering::Relaxed) {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        api.prevent_close();
                        quit(window.app_handle());
                    }
                }
            }
        })
        .setup(|app| {
            let controller = Controller::new(app.path().app_config_dir()?.join("settings.json"))
                .map_err(std::io::Error::other)?;
            let runtime = Runtime {
                controller,
                tray_available: Arc::new(AtomicBool::new(false)),
                menu_state: Default::default(),
                actions: Default::default(),
            };
            app.manage(runtime.clone());
            let locale = tauri::async_runtime::block_on(async {
                runtime
                    .controller
                    .data
                    .read()
                    .await
                    .settings
                    .language
                    .current()
            });
            let tray = TrayIconBuilder::with_id("lights")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/panel.png"
                ))?)
                .tooltip("Keylight Commander Linux")
                .menu(&Menu::with_items(
                    app,
                    &[&MenuItem::with_id(
                        app,
                        "open",
                        text(locale, "open_controls"),
                        true,
                        None::<&str>,
                    )?],
                )?)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref().to_string();
                    match id.as_str() {
                        "quit" => {
                            quit(app);
                            return;
                        }
                        "open" => {
                            show(app, false);
                            return;
                        }
                        "settings" => {
                            show(app, true);
                            return;
                        }
                        _ => {}
                    }
                    let app = app.clone();
                    let runtime = app.state::<Runtime>().inner().clone();
                    tauri::async_runtime::spawn(async move {
                        if id == "scan" {
                            let c = runtime.controller.clone();
                            let task =
                                tauri::async_runtime::spawn(async move { c.discover().await });
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                            publish(&app, &runtime).await;
                            let _ = task.await;
                        } else {
                            let action = runtime.actions.read().await.get(&id).cloned();
                            if let Some((target, patch)) = action {
                                let ids = runtime.controller.targets(target, false).await;
                                runtime.controller.change(ids, patch).await;
                            }
                        }
                        publish(&app, &runtime).await;
                    });
                })
                .build(app);
            let built = tray.is_ok();
            if let Err(e) = tray {
                eprintln!("Tray unavailable: {e}");
            }
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let host = tokio::task::spawn_blocking(has_tray_host)
                    .await
                    .unwrap_or(false);
                runtime
                    .tray_available
                    .store(built && host, Ordering::Relaxed);
                let first = !runtime.controller.data.read().await.settings.initialized;
                if first || !built || !host {
                    show(&app, false);
                }
                if first {
                    let autostart = app.autolaunch().enable();
                    if let Err(e) = autostart {
                        *runtime.controller.message.write().await =
                            Some(Message::new("error.autostart").cause(e.to_string()));
                        runtime.controller.data.write().await.settings.autostart = false;
                    }
                    runtime.controller.data.write().await.settings.initialized = true;
                    if let Err(e) = runtime.controller.save().await {
                        *runtime.controller.message.write().await = Some(e);
                    }
                }
                publish(&app, &runtime).await;
                let c = runtime.controller.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = c.discover().await;
                });
                let mut tick = 0u64;
                loop {
                    let visible = app
                        .get_webview_window("main")
                        .and_then(|w| w.is_visible().ok())
                        .unwrap_or(false);
                    if tick.is_multiple_of(5) {
                        let host = tokio::task::spawn_blocking(has_tray_host)
                            .await
                            .unwrap_or(false);
                        let was = runtime
                            .tray_available
                            .swap(built && host, Ordering::Relaxed);
                        if was && !host {
                            show(&app, false);
                        }
                    }
                    if visible || tick.is_multiple_of(5) {
                        runtime.controller.refresh(false).await;
                    }
                    publish(&app, &runtime).await;
                    tick += 1;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Unable to run Keylight Commander Linux");
}
