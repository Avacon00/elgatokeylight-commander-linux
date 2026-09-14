//! Shared, window-independent device controller. Only confirmed state is published.
use crate::i18n::{Language, Locale, Message};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, RwLock};

type Result<T> = std::result::Result<T, Message>;
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct LightState {
    pub on: u8,
    pub brightness: u8,
    pub temperature: u16,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Patch {
    pub on: Option<u8>,
    pub brightness: Option<u8>,
    pub temperature: Option<u16>,
}
impl Patch {
    pub fn apply(&self, state: &mut LightState) -> Result<()> {
        if self.on.is_some_and(|v| v > 1)
            || self.brightness.is_some_and(|v| v > 100)
            || self.temperature.is_some_and(|v| !(143..=344).contains(&v))
        {
            return Err(Message::new("error.invalid_setting"));
        }
        if let Some(v) = self.on {
            state.on = v;
        }
        if let Some(v) = self.brightness {
            state.brightness = v;
        }
        if let Some(v) = self.temperature {
            state.temperature = v;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Device {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub name: String,
    pub info: Value,
    #[serde(skip)]
    pub state: Option<LightState>,
    #[serde(skip)]
    pub error: Option<Message>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub language: Language,
    pub autostart: bool,
    pub sync: bool,
    pub initialized: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::System,
            autostart: true,
            sync: true,
            initialized: false,
        }
    }
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Saved {
    pub devices: Vec<Device>,
    pub settings: Settings,
}
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub locale: Locale,
    pub devices: Vec<DeviceView>,
    pub settings: Settings,
    pub scanning: bool,
    pub message: Option<Message>,
}
// Runtime state is intentionally excluded from the on-disk representation.
#[derive(Clone, Debug, Serialize)]
pub struct DeviceView {
    #[serde(flatten)]
    pub device: Device,
    pub state: Option<LightState>,
    pub error: Option<Message>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Outcome {
    pub succeeded: Vec<String>,
    pub failed: Vec<(String, Message)>,
}
impl Outcome {
    pub fn message(&self) -> Option<Message> {
        if self.failed.is_empty() {
            return None;
        }
        let mut message = Message::new("error.partial")
            .param("succeeded", self.succeeded.len())
            .param("failed", self.failed.len());
        for (name, error) in &self.failed {
            message = message.cause(
                Message::new("error.device")
                    .param("name", name)
                    .cause(error.clone()),
            );
        }
        Some(message)
    }
}
#[derive(Clone)]
pub struct Controller {
    operations: Arc<RwLock<()>>,
    closing: Arc<AtomicBool>,
    pub data: Arc<RwLock<Saved>>,
    pub scanning: Arc<Mutex<bool>>,
    pub settings_lock: Arc<Mutex<()>>,
    pub message: Arc<RwLock<Option<Message>>>,
    client: reqwest::Client,
    path: PathBuf,
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    attempted: Arc<Mutex<HashMap<String, Instant>>>,
    persist_lock: Arc<Mutex<()>>,
}
impl Controller {
    pub fn new(path: PathBuf) -> Result<Self> {
        let saved = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| Message::new("error.config_read").cause(e.to_string()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Saved::default(),
            Err(e) => return Err(e.to_string().into()),
        };
        Ok(Self {
            operations: Default::default(),
            closing: Default::default(),
            data: Arc::new(RwLock::new(saved)),
            scanning: Arc::new(Mutex::new(false)),
            settings_lock: Default::default(),
            message: Arc::new(RwLock::new(None)),
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(4))
                .no_proxy()
                .build()
                .map_err(|e| e.to_string())?,
            path,
            locks: Default::default(),
            attempted: Default::default(),
            persist_lock: Default::default(),
        })
    }
    async fn operation(&self) -> Result<tokio::sync::OwnedRwLockReadGuard<()>> {
        let guard = self.operations.clone().read_owned().await;
        if self.closing.load(Ordering::Relaxed) {
            return Err(Message::new("error.shutting_down"));
        }
        Ok(guard)
    }
    pub async fn finish_operations(&self) {
        self.closing.store(true, Ordering::Relaxed);
        let _guard = self.operations.write().await;
    }
    pub async fn update_settings(
        &self,
        autostart: Option<bool>,
        sync: Option<bool>,
        language: Option<Language>,
        mut configure_autostart: impl FnMut(bool) -> Result<()>,
    ) -> Result<()> {
        let _operation = self.operation().await?;
        let _settings_guard = self.settings_lock.lock().await;
        let _persist_guard = self.persist_lock.lock().await;
        let mut data = self.data.write().await;
        let old = data.settings.clone();
        let changed_autostart = autostart.filter(|v| *v != old.autostart);
        if let Some(enabled) = changed_autostart {
            configure_autostart(enabled)?;
        }
        if let Some(value) = autostart {
            data.settings.autostart = value;
        }
        if let Some(value) = sync {
            data.settings.sync = value;
        }
        if let Some(value) = language {
            data.settings.language = value;
        }
        if let Err(error) = self.write_saved(&data) {
            data.settings = old;
            if let Some(enabled) = changed_autostart {
                if let Err(rollback) = configure_autostart(data.settings.autostart) {
                    // Keep the displayed value aligned with the last successful OS change.
                    data.settings.autostart = enabled;
                    return Err(error.cause(rollback));
                }
            }
            return Err(error);
        }
        Ok(())
    }
    pub async fn snapshot(&self) -> Snapshot {
        let data = self.data.read().await;
        Snapshot {
            locale: data.settings.language.current(),
            devices: data
                .devices
                .iter()
                .map(|d| DeviceView {
                    device: d.clone(),
                    state: d.state.clone(),
                    error: d.error.clone(),
                })
                .collect(),
            settings: data.settings.clone(),
            scanning: *self.scanning.lock().await,
            message: self.message.read().await.clone(),
        }
    }
    pub async fn save(&self) -> Result<()> {
        let _guard = self.persist_lock.lock().await;
        self.write_saved(&*self.data.read().await)
    }
    fn write_saved(&self, data: &Saved) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| Message::new("error.config_directory"))?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = self.path.with_extension("tmp");
        std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(temp, &self.path)
            .map_err(|e| Message::new("error.config_save").cause(e.to_string()))
    }
    async fn gate(&self, id: &str) -> Arc<Mutex<()>> {
        self.locks
            .lock()
            .await
            .entry(id.into())
            .or_default()
            .clone()
    }
    async fn device(&self, id: &str) -> Result<Device> {
        self.data
            .read()
            .await
            .devices
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or(Message::new("error.unknown_light"))
    }
    fn url(d: &Device, path: &str) -> String {
        let host = if d.host.contains(':') {
            format!("[{}]", d.host)
        } else {
            d.host.clone()
        };
        format!("http://{host}:{}/elgato/{path}", d.port)
    }
    async fn read(&self, d: &Device) -> Result<LightState> {
        #[derive(Deserialize)]
        struct Response {
            lights: Vec<LightState>,
        }
        let value: Response = self
            .client
            .get(Self::url(d, "lights"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        let state = value
            .lights
            .into_iter()
            .next()
            .ok_or_else(|| Message::new("error.no_lights"))?;
        Patch {
            on: Some(state.on),
            brightness: Some(state.brightness),
            temperature: Some(state.temperature),
        }
        .apply(&mut state.clone())?;
        Ok(state)
    }
    async fn write(&self, d: &Device, state: &LightState) -> Result<LightState> {
        self.client
            .put(Self::url(d, "lights"))
            .json(&json!({"numberOfLights":1,"lights":[state]}))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        self.read(d).await
    }
    async fn record(&self, id: &str, result: &Result<LightState>) {
        self.attempted
            .lock()
            .await
            .insert(id.into(), Instant::now());
        if let Some(d) = self
            .data
            .write()
            .await
            .devices
            .iter_mut()
            .find(|d| d.id == id)
        {
            match result {
                Ok(s) => {
                    d.state = Some(s.clone());
                    d.error = None;
                }
                Err(e) => {
                    d.error = Some(e.clone());
                }
            }
        }
    }
    pub async fn refresh(&self, force: bool) {
        let devices = self.data.read().await.devices.clone();
        let mut tasks = tokio::task::JoinSet::new();
        for d in devices {
            if !force
                && d.error.is_some()
                && self
                    .attempted
                    .lock()
                    .await
                    .get(&d.id)
                    .is_some_and(|t| t.elapsed() < Duration::from_secs(30))
            {
                continue;
            }
            let this = self.clone();
            tasks.spawn(async move {
                let gate = this.gate(&d.id).await;
                let Ok(_guard) = gate.try_lock() else {
                    return;
                };
                // Resolve again inside the lock, as discovery may have changed the address.
                if let Ok(current) = this.device(&d.id).await {
                    let result = this.read(&current).await;
                    this.record(&d.id, &result).await;
                }
            });
        }
        while tasks.join_next().await.is_some() {}
    }
    pub async fn change(&self, ids: Vec<String>, patch: Patch) -> Outcome {
        let _operation = match self.operation().await {
            Ok(g) => g,
            Err(e) => {
                return Outcome {
                    succeeded: vec![],
                    failed: vec![("".into(), e)],
                }
            }
        };
        let mut tasks = tokio::task::JoinSet::new();
        let mut ids = ids;
        ids.sort();
        ids.dedup();
        for id in ids {
            let this = self.clone();
            let patch = patch.clone();
            tasks.spawn(async move {
                let gate = this.gate(&id).await;
                let _guard = gate.lock().await;
                let result = async {
                    let d = this.device(&id).await?;
                    let mut current = this.read(&d).await?;
                    patch.apply(&mut current)?;
                    this.write(&d, &current).await
                }
                .await;
                this.record(&id, &result).await;
                (id, result)
            });
        }
        let mut outcome = Outcome::default();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((id, Ok(_))) => outcome.succeeded.push(id),
                Ok((id, Err(e))) => outcome.failed.push((id, e)),
                Err(e) => outcome.failed.push(("".into(), e.to_string().into())),
            }
        }
        *self.message.write().await = outcome.message();
        outcome
    }
    pub async fn targets(&self, id: Option<String>, sync: bool) -> Vec<String> {
        let data = self.data.read().await;
        if id.is_none() || (sync && data.settings.sync) {
            data.devices
                .iter()
                .filter(|d| d.error.is_none() && d.state.is_some())
                .map(|d| d.id.clone())
                .collect()
        } else {
            id.into_iter().collect()
        }
    }
    pub async fn discover(&self) -> Result<()> {
        {
            let mut running = self.scanning.lock().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }
        let result = self.discover_inner().await;
        *self.scanning.lock().await = false;
        if let Err(e) = &result {
            *self.message.write().await = Some(Message::new("error.scan").cause(e.clone()));
        }
        result
    }
    async fn discover_inner(&self) -> Result<()> {
        let found = tokio::task::spawn_blocking(|| -> Result<Vec<(String, u16)>> {
            let daemon = mdns_sd::ServiceDaemon::new().map_err(|e| e.to_string())?;
            let receiver = match daemon.browse("_elg._tcp.local.") {
                Ok(r) => r,
                Err(e) => {
                    let _ = daemon.shutdown();
                    return Err(e.to_string().into());
                }
            };
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut found = Vec::new();
            while Instant::now() < deadline {
                if let Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) =
                    receiver.recv_timeout(Duration::from_millis(100))
                {
                    let host = info
                        .get_addresses_v4()
                        .iter()
                        .next()
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| info.get_hostname().trim_end_matches('.').to_string());
                    let endpoint = (host, info.get_port());
                    if !found.contains(&endpoint) {
                        found.push(endpoint);
                    }
                }
            }
            let _ = daemon.shutdown();
            Ok(found)
        })
        .await
        .map_err(|e| e.to_string())??;
        let mut errors = Vec::new();
        for (host, port) in found {
            if let Err(e) = self.add_endpoint(host, port).await {
                errors.push(e);
            }
        }
        self.refresh(true).await;
        self.save().await?;
        *self.message.write().await = if errors.is_empty() {
            None
        } else {
            Some(Message {
                causes: errors,
                ..Message::new("error.discovery")
            })
        };
        Ok(())
    }
    pub async fn add_endpoint(&self, host: String, port: u16) -> Result<()> {
        let mut d = Device {
            id: String::new(),
            host,
            port,
            name: String::new(),
            info: Value::Null,
            state: None,
            error: None,
        };
        let info: Value = self
            .client
            .get(Self::url(&d, "accessory-info"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        d.id = identity(&info).ok_or_else(|| Message::new("error.no_identity"))?;
        d.name = info
            .get("displayName")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("Key Light")
            .into();
        d.info = info;
        let gate = self.gate(&d.id).await;
        let _guard = gate.lock().await;
        let state = self.read(&d).await;
        match state {
            Ok(s) => d.state = Some(s),
            Err(e) => d.error = Some(e),
        };
        merge(&mut self.data.write().await.devices, d);
        Ok(())
    }
    pub async fn remove(&self, id: &str) -> Result<()> {
        let _operation = self.operation().await?;
        let gate = self.gate(id).await;
        let _guard = gate.lock().await;
        let _persist_guard = self.persist_lock.lock().await;
        let mut data = self.data.write().await;
        if let Some(index) = data.devices.iter().position(|d| d.id == id) {
            let removed = data.devices.remove(index);
            if let Err(error) = self.write_saved(&data) {
                data.devices.insert(index, removed);
                return Err(error);
            }
        }
        Ok(())
    }
    pub async fn rename(&self, id: &str, name: String) -> Result<()> {
        let _operation = self.operation().await?;
        if name.trim().is_empty() || name.len() > 128 {
            return Err(Message::new("error.name"));
        }
        let gate = self.gate(id).await;
        let _guard = gate.lock().await;
        let d = self.device(id).await?;
        self.client
            .put(Self::url(&d, "accessory-info"))
            .json(&json!({"displayName":name}))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let info: Value = self
            .client
            .get(Self::url(&d, "accessory-info"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        if let Some(d) = self
            .data
            .write()
            .await
            .devices
            .iter_mut()
            .find(|d| d.id == id)
        {
            d.name = info["displayName"].as_str().unwrap_or(&name).into();
            d.info = info;
        }
        self.save().await
    }
    pub async fn identify(&self, id: &str) -> Result<()> {
        let _operation = self.operation().await?;
        let gate = self.gate(id).await;
        let _guard = gate.lock().await;
        let d = self.device(id).await?;
        let original = self.read(&d).await?;
        let result = async {
            for _ in 0..3 {
                for brightness in [50, 0] {
                    let mut s = original.clone();
                    s.on = 1;
                    s.brightness = brightness;
                    self.write(&d, &s).await?;
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
            Ok::<(), Message>(())
        }
        .await;
        // Always attempt restoration, including after a failed blink request.
        let mut restored = self.write(&d, &original).await;
        if restored.is_err() {
            restored = self.write(&d, &original).await;
        }
        self.record(id, &restored).await;
        match (result, restored) {
            (_, Err(e)) => Err(Message::new("error.restore").cause(e)),
            (Err(e), _) => Err(e),
            _ => Ok(()),
        }
    }
}
pub fn identity(info: &Value) -> Option<String> {
    ["serialNumber", "macAddress"].iter().find_map(|key| {
        info.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("{key}:{}", s.to_ascii_lowercase()))
    })
}
pub fn merge(devices: &mut Vec<Device>, device: Device) {
    if let Some(old) = devices.iter_mut().find(|d| d.id == device.id) {
        *old = device;
    } else {
        devices.push(device);
    }
    devices.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Mutex as StdMutex,
        },
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fake {
        port: u16,
        state: Arc<StdMutex<LightState>>,
        writes: Arc<StdMutex<Vec<LightState>>>,
        fail_put: Arc<AtomicBool>,
        fail_get: Arc<AtomicBool>,
        slow: Arc<AtomicBool>,
        stop: Arc<AtomicBool>,
    }
    impl Fake {
        fn new(serial: &str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            listener.set_nonblocking(true).unwrap();
            let state = Arc::new(StdMutex::new(LightState {
                on: 0,
                brightness: 35,
                temperature: 200,
            }));
            let writes = Arc::new(StdMutex::new(Vec::new()));
            let fail_put = Arc::new(AtomicBool::new(false));
            let fail_get = Arc::new(AtomicBool::new(false));
            let slow = Arc::new(AtomicBool::new(false));
            let stop = Arc::new(AtomicBool::new(false));
            let (s, w, fp, fg, sl, st) = (
                state.clone(),
                writes.clone(),
                fail_put.clone(),
                fail_get.clone(),
                slow.clone(),
                stop.clone(),
            );
            let serial = serial.to_string();
            std::thread::spawn(move || {
                while !st.load(Ordering::Relaxed) {
                    let Ok((mut stream, _)) = listener.accept() else {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    };
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    let mut buf = [0u8; 4096];
                    let header_end;
                    loop {
                        let n = stream.read(&mut buf).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        bytes.extend_from_slice(&buf[..n]);
                        if let Some(i) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                            header_end = i + 4;
                            let head = String::from_utf8_lossy(&bytes[..i]);
                            let len = head
                                .lines()
                                .find_map(|l| {
                                    l.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .and_then(|v| v.trim().parse::<usize>().ok())
                                })
                                .unwrap_or(0);
                            while bytes.len() < header_end + len {
                                let n = stream.read(&mut buf).unwrap_or(0);
                                if n == 0 {
                                    break;
                                }
                                bytes.extend_from_slice(&buf[..n]);
                            }
                            break;
                        }
                    }
                    let text = String::from_utf8_lossy(&bytes);
                    let put = text.starts_with("PUT");
                    let info = text.lines().next().unwrap_or("").contains("accessory-info");
                    if sl.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_secs(5));
                    }
                    let fail = if put {
                        fp.swap(false, Ordering::Relaxed)
                    } else {
                        fg.load(Ordering::Relaxed)
                    };
                    let body=if info {json!({"serialNumber":serial,"displayName":"Test light","macAddress":serial,"productName":"Key Light"})} else {
                    if put && !fail {let body=text.split_once("\r\n\r\n").unwrap().1;let value:Value=serde_json::from_str(body).unwrap();let next:LightState=serde_json::from_value(value["lights"][0].clone()).unwrap();w.lock().unwrap().push(next.clone());*s.lock().unwrap()=next;}
                    json!({"numberOfLights":1,"lights":[*s.lock().unwrap()]})
                }.to_string();
                    let response=format!("HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",if fail {"500 Error"}else{"200 OK"},body.len(),body);
                    let _ = stream.write_all(response.as_bytes());
                }
            });
            Self {
                port,
                state,
                writes,
                fail_put,
                fail_get,
                slow,
                stop,
            }
        }
    }
    impl Drop for Fake {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }
    fn controller() -> Controller {
        Controller::new(std::env::temp_dir().join(format!(
            "keylight-tests-{}-{}/settings.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
        .unwrap()
    }
    async fn add(c: &Controller, f: &Fake) -> String {
        c.add_endpoint("127.0.0.1".into(), f.port).await.unwrap();
        c.data
            .read()
            .await
            .devices
            .iter()
            .find(|d| d.port == f.port)
            .unwrap()
            .id
            .clone()
    }
    #[tokio::test]
    async fn patches_preserve_other_fields_and_serialize_concurrent_changes() {
        let f = Fake::new("one");
        let c = controller();
        let id = add(&c, &f).await;
        let (a, b) = tokio::join!(
            c.change(
                vec![id.clone()],
                Patch {
                    brightness: Some(75),
                    ..Default::default()
                }
            ),
            c.change(
                vec![id.clone()],
                Patch {
                    on: Some(1),
                    ..Default::default()
                }
            )
        );
        assert!(a.failed.is_empty() && b.failed.is_empty());
        assert_eq!(
            *f.state.lock().unwrap(),
            LightState {
                on: 1,
                brightness: 75,
                temperature: 200
            }
        );
        assert_eq!(f.writes.lock().unwrap().len(), 2);
    }
    #[tokio::test]
    async fn groups_report_partial_failure_and_never_confirm_failed_write() {
        let a = Fake::new("one");
        let b = Fake::new("two");
        let c = controller();
        let aid = add(&c, &a).await;
        let bid = add(&c, &b).await;
        b.fail_put.store(true, Ordering::Relaxed);
        let out = c
            .change(
                vec![aid, bid.clone()],
                Patch {
                    on: Some(1),
                    ..Default::default()
                },
            )
            .await;
        assert_eq!(out.succeeded.len(), 1);
        assert_eq!(out.failed.len(), 1);
        let d = c.device(&bid).await.unwrap();
        assert!(d.error.is_some());
        assert_eq!(d.state.unwrap().on, 0);
        assert_eq!(b.state.lock().unwrap().on, 0);
    }
    #[tokio::test]
    async fn rediscovery_merges_identity_updates_endpoint_and_preserves_missing_devices() {
        let a = Fake::new("one");
        let b = Fake::new("two");
        let replacement = Fake::new("one");
        let c = controller();
        let id = add(&c, &a).await;
        add(&c, &b).await;
        add(&c, &a).await;
        add(&c, &replacement).await;
        assert_eq!(c.data.read().await.devices.len(), 2);
        assert_eq!(c.device(&id).await.unwrap().port, replacement.port);
    }
    #[tokio::test]
    async fn persisted_settings_survive_restart_but_runtime_state_does_not() {
        let f = Fake::new("one");
        let c = controller();
        let id = add(&c, &f).await;
        c.data.write().await.settings = Settings {
            language: Language::System,
            autostart: false,
            sync: false,
            initialized: true,
        };
        c.save().await.unwrap();
        let restored = Controller::new(c.path.clone()).unwrap();
        let data = restored.data.read().await;
        assert!(!data.settings.sync);
        assert!(!data.settings.autostart);
        assert!(data.settings.initialized);
        assert_eq!(data.devices[0].id, id);
        assert!(data.devices[0].state.is_none());
        std::fs::remove_dir_all(c.path.parent().unwrap()).unwrap();
    }
    #[tokio::test]
    async fn failed_reads_mark_offline_and_refresh_recovers() {
        let f = Fake::new("one");
        let c = controller();
        let id = add(&c, &f).await;
        f.fail_get.store(true, Ordering::Relaxed);
        c.refresh(true).await;
        assert!(c.device(&id).await.unwrap().error.is_some());
        f.fail_get.store(false, Ordering::Relaxed);
        c.refresh(true).await;
        assert!(c.device(&id).await.unwrap().error.is_none());
    }
    #[tokio::test]
    async fn request_timeout_is_bounded() {
        let f = Fake::new("one");
        let c = controller();
        let id = add(&c, &f).await;
        f.slow.store(true, Ordering::Relaxed);
        let start = Instant::now();
        let result = c
            .change(
                vec![id],
                Patch {
                    on: Some(1),
                    ..Default::default()
                },
            )
            .await;
        assert_eq!(result.failed.len(), 1);
        assert!(start.elapsed() < Duration::from_secs(5));
    }
    #[tokio::test]
    async fn identify_restores_state_even_if_a_blink_write_fails() {
        let f = Fake::new("one");
        let c = controller();
        let id = add(&c, &f).await;
        let before = f.state.lock().unwrap().clone();
        f.fail_put.store(true, Ordering::Relaxed);
        assert!(c.identify(&id).await.is_err());
        assert_eq!(*f.state.lock().unwrap(), before);
        assert!(c.device(&id).await.unwrap().error.is_none());
    }
    #[tokio::test]
    async fn panel_single_target_ignores_window_sync() {
        let a = Fake::new("one");
        let b = Fake::new("two");
        let c = controller();
        let id = add(&c, &a).await;
        add(&c, &b).await;
        assert_eq!(c.targets(Some(id.clone()), false).await, vec![id.clone()]);
        assert_eq!(c.targets(Some(id), true).await.len(), 2);
    }
    #[tokio::test]
    async fn quit_waits_for_identification_restore_and_rejects_new_changes() {
        let f = Fake::new("one");
        let c = controller();
        let id = add(&c, &f).await;
        let before = f.state.lock().unwrap().clone();
        let task_controller = c.clone();
        let task_id = id.clone();
        let task = tokio::spawn(async move { task_controller.identify(&task_id).await });
        while f.writes.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        c.finish_operations().await;
        task.await.unwrap().unwrap();
        assert_eq!(*f.state.lock().unwrap(), before);
        let out = c
            .change(
                vec![id],
                Patch {
                    on: Some(1),
                    ..Default::default()
                },
            )
            .await;
        assert_eq!(out.failed.len(), 1);
        assert_eq!(*f.state.lock().unwrap(), before);
    }
    #[tokio::test]
    async fn old_configuration_defaults_to_system_and_language_only_changes_are_isolated() {
        let c = controller();
        std::fs::create_dir_all(c.path.parent().unwrap()).unwrap();
        std::fs::write(
            &c.path,
            r#"{"devices":[],"settings":{"autostart":false,"sync":false,"initialized":true}}"#,
        )
        .unwrap();
        let c = Controller::new(c.path.clone()).unwrap();
        assert_eq!(c.snapshot().await.settings.language, Language::System);
        *c.message.write().await = Some(Message::new("error.invalid_setting"));
        for language in [Language::De, Language::En, Language::System] {
            c.update_settings(None, None, Some(language), |_| {
                panic!("Language must not configure autostart")
            })
            .await
            .unwrap();
            let restored = Controller::new(c.path.clone()).unwrap();
            let snapshot = restored.snapshot().await;
            assert_eq!(snapshot.settings.language, language);
            assert!(!snapshot.settings.autostart);
            assert!(!snapshot.settings.sync);
            assert!(snapshot.settings.initialized);
            assert_eq!(
                c.snapshot().await.message.unwrap().key,
                "error.invalid_setting"
            );
        }
        std::fs::remove_dir_all(c.path.parent().unwrap()).unwrap();
    }
    #[tokio::test]
    async fn partial_settings_preserve_language_and_only_configure_changed_autostart() {
        let c = controller();
        c.update_settings(None, Some(false), Some(Language::De), |_| {
            panic!("No autostart change requested")
        })
        .await
        .unwrap();
        c.update_settings(Some(true), None, None, |_| panic!("Already enabled"))
            .await
            .unwrap();
        c.update_settings(Some(false), None, None, |enabled| {
            assert!(!enabled);
            Ok(())
        })
        .await
        .unwrap();
        let snap = c.snapshot().await;
        assert_eq!(snap.locale, Locale::De);
        assert_eq!(snap.settings.language, Language::De);
        assert!(!snap.settings.sync);
        assert!(!snap.settings.autostart);
        c.update_settings(Some(true), Some(true), Some(Language::En), |_| {
            Err(Message::new("error.autostart"))
        })
        .await
        .unwrap_err();
        let snap = c.snapshot().await;
        assert_eq!(snap.locale, Locale::De);
        assert!(!snap.settings.sync);
        assert!(!snap.settings.autostart);
        std::fs::remove_dir_all(c.path.parent().unwrap()).unwrap();
    }
    #[tokio::test]
    async fn failed_language_save_rolls_back_memory_without_autostart_side_effects() {
        let c = controller();
        std::fs::write(c.path.parent().unwrap(), b"not a directory").unwrap();
        c.update_settings(None, None, Some(Language::De), |_| {
            panic!("No autostart changes")
        })
        .await
        .unwrap_err();
        assert_eq!(c.snapshot().await.settings.language, Language::System);
        std::fs::remove_file(c.path.parent().unwrap()).unwrap();
    }
    #[tokio::test]
    async fn failed_autostart_save_restores_os_and_all_settings() {
        let c = controller();
        c.save().await.unwrap();
        std::fs::create_dir(c.path.with_extension("tmp")).unwrap();
        let mut calls = vec![];
        c.update_settings(Some(false), Some(false), Some(Language::De), |enabled| {
            calls.push(enabled);
            Ok(())
        })
        .await
        .unwrap_err();
        assert_eq!(calls, vec![false, true]);
        let snapshot = c.snapshot().await;
        assert!(snapshot.settings.autostart);
        assert!(snapshot.settings.sync);
        assert_eq!(snapshot.settings.language, Language::System);
        assert!(
            Controller::new(c.path.clone())
                .unwrap()
                .snapshot()
                .await
                .settings
                .autostart
        );
        std::fs::remove_dir_all(c.path.parent().unwrap()).unwrap();
    }
    #[tokio::test]
    async fn failed_autostart_rollback_reports_both_errors_and_actual_setting() {
        let c = controller();
        std::fs::write(c.path.parent().unwrap(), b"not a directory").unwrap();
        let error = c
            .update_settings(Some(false), None, None, |enabled| {
                if enabled {
                    Err(Message::new("error.autostart"))
                } else {
                    Ok(())
                }
            })
            .await
            .unwrap_err();
        assert!(!c.snapshot().await.settings.autostart);
        assert!(error
            .causes
            .iter()
            .any(|cause| cause.key == "error.autostart"));
        std::fs::remove_file(c.path.parent().unwrap()).unwrap();
    }
    #[tokio::test]
    async fn failed_removal_preserves_device_in_memory_and_on_disk_then_retries() {
        let f = Fake::new("remove");
        let c = controller();
        let id = add(&c, &f).await;
        c.save().await.unwrap();
        std::fs::create_dir(c.path.with_extension("tmp")).unwrap();
        c.remove(&id).await.unwrap_err();
        let snapshot = c.snapshot().await;
        assert_eq!(snapshot.devices.len(), 1);
        assert!(snapshot.devices[0].state.is_some());
        assert_eq!(
            Controller::new(c.path.clone())
                .unwrap()
                .snapshot()
                .await
                .devices
                .len(),
            1
        );
        std::fs::remove_dir(c.path.with_extension("tmp")).unwrap();
        c.remove(&id).await.unwrap();
        assert!(c.snapshot().await.devices.is_empty());
        assert!(Controller::new(c.path.clone())
            .unwrap()
            .snapshot()
            .await
            .devices
            .is_empty());
        std::fs::remove_dir_all(c.path.parent().unwrap()).unwrap();
    }
    #[test]
    fn identity_falls_back_to_mac_and_invalid_values_are_rejected() {
        assert_eq!(
            identity(&json!({"serialNumber":"","macAddress":"AA:BB"})),
            Some("macAddress:aa:bb".into())
        );
        assert!(Patch {
            brightness: Some(101),
            ..Default::default()
        }
        .apply(&mut LightState::default())
        .is_err());
    }
}
