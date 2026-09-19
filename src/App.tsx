import { createContext, useContext, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
	Power,
	Settings as SettingsIcon,
	ScanEye,
	ArrowLeft,
	Sun,
	Thermometer,
	MoreHorizontal,
} from "lucide-react";
import { Button } from "./components/ui/button";
import { Slider, TemperatureSlider } from "./components/ui/slider";
import "./styles.css";
import { createCoalescer } from "./lib/coalescer";

import {
	t,
	renderMessage,
	commandError,
	type Locale,
	type Language,
	type Message,
	type TextKey,
} from "./lib/i18n";

const LocaleContext = createContext<Locale>("en");
function useTranslations() {
	const locale = useContext(LocaleContext);
	return (key: TextKey, params?: Record<string, string | number>) =>
		t(locale, key, params);
}

type LightState = { on: number; brightness: number; temperature: number };
type Device = {
	id: string;
	name: string;
	host: string;
	port: number;
	info: Record<string, unknown>;
	state: LightState | null;
	error: Message | null;
};
type Snapshot = {
	devices: Device[];
	settings: { autostart: boolean; sync: boolean; language: Language };
	locale: Locale;
	scanning: boolean;
	message: Message | null;
};
type Outcome = { succeeded: string[]; failed: [string, Message][] };
type Patch = Partial<LightState>;

export default function App() {
	const [snapshot, setSnapshot] = useState<Snapshot>();
	const locale: Locale =
		snapshot?.locale ??
		(navigator.language.split(/[-_]/)[0].toLowerCase() === "de" ? "de" : "en");
	const tr = (key: TextKey, params?: Record<string, string | number>) =>
		t(locale, key, params);
	useEffect(() => {
		document.documentElement.lang = locale;
	}, [locale]);
	const [page, setPage] = useState("lights");
	const [error, setError] = useState<Message | null>(null);
	const [busy, setBusy] = useState(false);
	const mounted = useRef(true);
	useEffect(() => {
		mounted.current = true;
		let revision = 0;
		const changes = listen<Snapshot>("lights-changed", (e) => {
			revision++;
			if (mounted.current) setSnapshot(e.payload);
		});
		const navigation = listen<string>("navigate", (e) => setPage(e.payload));
		void changes
			.then(async () => {
				const version = revision;
				const s = await invoke<Snapshot>("snapshot");
				if (mounted.current && version === revision) setSnapshot(s);
			})
			.catch((e) => setError(commandError(e)));
		return () => {
			mounted.current = false;
			void changes.then((f) => f());
			void navigation.then((f) => f());
		};
	}, []);
	async function run<T>(
		command: string,
		args?: Record<string, unknown>,
	): Promise<T | undefined> {
		if (
			!(
				command === "update_settings" &&
				args &&
				Object.keys(args).length === 1 &&
				"language" in args
			)
		)
			setError(null);
		try {
			return await invoke<T>(command, args);
		} catch (e) {
			setError(commandError(e));
			return undefined;
		}
	}
	async function change(id: string | null, patch: Patch, sync = true) {
		const result = await run<Outcome>("change_light", { id, patch, sync });
		if (result?.failed.length)
			setError({
				key: "error.partial",
				params: {
					succeeded: result.succeeded.length,
					failed: result.failed.length,
				},
				causes: result.failed.map(([id, error]) => ({
					key: "error.device",
					params: {
						name: snapshot?.devices.find((d) => d.id === id)?.name ?? id,
					},
					causes: [error],
				})),
			});
	}
	const selected = snapshot?.devices.find((d) => d.id === page);
	const online = snapshot?.devices.filter((d) => d.state && !d.error) ?? [];
	return (
		<LocaleContext.Provider value={locale}>
			<div className="min-h-screen bg-neutral-950 text-neutral-100 text-sm">
				<header className="sticky top-0 z-10 flex items-center justify-between gap-2 bg-neutral-900 p-3 border-b border-neutral-800">
					{page === "lights" ? (
						<Button
							aria-label={tr("a11y.toggle_all")}
							title={tr("a11y.toggle_all")}
							className="shrink-0"
							size="icon-sm"
							disabled={!online.length}
							onClick={() =>
								void change(
									null,
									{ on: online.every((d) => d.state?.on === 1) ? 0 : 1 },
									false,
								)
							}
						>
							<Power />
						</Button>
					) : (
						<Button
							aria-label={tr("back")}
							className="shrink-0"
							size="icon-sm"
							variant="ghost"
							onClick={() => setPage("lights")}
						>
							<ArrowLeft />
						</Button>
					)}
					<span
						className="min-w-0 flex-1 truncate font-medium"
						title={selected?.name}
					>
						{page === "settings"
							? tr("settings")
							: selected
								? selected.name
								: "Keylight Commander Linux"}
					</span>
					<div className="flex shrink-0 gap-1">
						<Button
							aria-label={tr("scan")}
							title={tr("scan")}
							className="shrink-0"
							size="icon-sm"
							variant="ghost"
							disabled={snapshot?.scanning || busy}
							onClick={async () => {
								setBusy(true);
								await run("scan");
								setBusy(false);
							}}
						>
							<ScanEye className={snapshot?.scanning ? "animate-pulse" : ""} />
						</Button>
						<Button
							aria-label={tr("settings")}
							className="shrink-0"
							size="icon-sm"
							variant="ghost"
							onClick={() => setPage("settings")}
						>
							<SettingsIcon />
						</Button>
					</div>
				</header>
				{(error || snapshot?.message) && (
					<div
						role="alert"
						className="m-3 p-3 bg-red-950 text-red-200 rounded break-words"
					>
						{renderMessage(locale, (error || snapshot?.message)!)}
					</div>
				)}
				{!snapshot && <p className="p-6">{tr("loading")}</p>}
				{snapshot && page === "settings" && (
					<section className="p-4 space-y-5">
						<label className="flex flex-col gap-2">
							<span>{tr("language")}</span>
							<select
								className="language-select"
								disabled={busy}
								value={snapshot.settings.language}
								onChange={async (e) => {
									setBusy(true);
									await run("update_settings", { language: e.target.value });
									setBusy(false);
								}}
							>
								<option value="system">{tr("system_language")}</option>
								<option value="de">Deutsch</option>
								<option value="en">English</option>
							</select>
						</label>
						<label className="flex items-start gap-3">
							<input
								type="checkbox"
								disabled={busy}
								checked={snapshot.settings.autostart}
								onChange={async (e) => {
									setBusy(true);
									await run("update_settings", {
										autostart: e.target.checked,
									});
									setBusy(false);
								}}
							/>
							<span>
								{tr("autostart")}
								<p className="text-neutral-400 mt-1">{tr("autostart_help")}</p>
							</span>
						</label>
						<label className="flex items-start gap-3">
							<input
								type="checkbox"
								disabled={busy}
								checked={snapshot.settings.sync}
								onChange={async (e) => {
									setBusy(true);
									await run("update_settings", {
										sync: e.target.checked,
									});
									setBusy(false);
								}}
							/>
							<span>
								{tr("sync")}
								<p className="text-neutral-400 mt-1">{tr("sync_help")}</p>
							</span>
						</label>
						<p className="text-neutral-400">{tr("close_help")}</p>
						<p className="text-neutral-500">
							{tr("version", { version: "0.2.4" })}
						</p>
					</section>
				)}
				{snapshot && page === "lights" && (
					<main className="p-3 space-y-3">
						{!snapshot.devices.length && (
							<div className="py-12 text-center space-y-3">
								<p>{snapshot.scanning ? tr("searching") : tr("no_lights")}</p>
								<p className="text-neutral-400">{tr("connect_hint")}</p>
								<Button
									disabled={snapshot.scanning || busy}
									onClick={async () => {
										setBusy(true);
										await run("scan");
										setBusy(false);
									}}
								>
									{tr("scan")}
								</Button>
							</div>
						)}
						{snapshot.devices.map((d) => (
							<Light
								key={d.id}
								device={d}
								onChange={(patch) => change(d.id, patch)}
								onDetails={() => setPage(d.id)}
							/>
						))}
						{!!snapshot.devices.length && (
							<p className="text-xs text-neutral-500">
								{snapshot.settings.sync ? tr("synced") : tr("not_synced")}
							</p>
						)}
					</main>
				)}
				{selected && (
					<section className="p-4 space-y-5">
						<Light
							device={selected}
							onChange={(patch) => change(selected.id, patch)}
							onDetails={() => {}}
						/>
						<NameEditor
							key={selected.id + selected.name}
							device={selected}
							save={async (name) => {
								await run("rename_light", { id: selected.id, name });
							}}
						/>
						<dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 break-all">
							{[
								[tr("model"), selected.info.productName],
								[tr("serial"), selected.info.serialNumber],
								[tr("firmware"), selected.info.firmwareVersion],
								[tr("mac"), selected.info.macAddress],
								[tr("address"), `${selected.host}:${selected.port}`],
							].map(([label, value]) => (
								<div key={String(label)} className="contents">
									<dt className="text-neutral-400">{String(label)}</dt>
									<dd>{String(value ?? "—")}</dd>
								</div>
							))}
						</dl>
						<div className="flex gap-3">
							<Button
								disabled={busy || !!selected.error || !selected.state}
								onClick={async () => {
									setBusy(true);
									await run("identify_light", { id: selected.id });
									setBusy(false);
								}}
							>
								{tr("identify")}
							</Button>
							<Button
								variant="secondary"
								disabled={busy}
								onClick={async () => {
									await run("remove_light", { id: selected.id });
									setPage("lights");
								}}
							>
								{tr("remove")}
							</Button>
						</div>
					</section>
				)}
			</div>
		</LocaleContext.Provider>
	);
}
function NameEditor({
	device,
	save,
}: { device: Device; save: (name: string) => Promise<void> }) {
	const tr = useTranslations();
	const [name, setName] = useState(device.name);
	const [busy, setBusy] = useState(false);
	return (
		<form
			className="flex gap-2"
			onSubmit={async (e) => {
				e.preventDefault();
				setBusy(true);
				await save(name);
				setBusy(false);
			}}
		>
			<input
				aria-label={tr("light_name")}
				className="min-w-0 flex-1 bg-neutral-900 border border-neutral-700 rounded p-2"
				value={name}
				maxLength={128}
				onChange={(e) => setName(e.target.value)}
			/>
			<Button disabled={busy || !name.trim() || name === device.name}>
				{tr("save_name")}
			</Button>
		</form>
	);
}
function Light({
	device,
	onChange,
	onDetails,
}: {
	device: Device;
	onChange: (p: Patch) => Promise<void>;
	onDetails: () => void;
}) {
	const tr = useTranslations();
	const locale = useContext(LocaleContext);
	// Keep a local preview while dragging. Coalesce to one in-flight request plus the
	// newest pending values, so slow devices cannot accumulate stale slider writes.
	const [draft, setDraft] = useState<Patch>({});
	const alive = useRef(true);
	const send = useRef(onChange);
	send.current = onChange;
	const [queueController] = useState(() =>
		createCoalescer<Patch>(
			(patch) => send.current(patch),
			() => {
				if (alive.current) setDraft({});
			},
			() => {
				if (alive.current) setDraft({});
			},
		),
	);
	function queue(patch: Patch) {
		setDraft((old) => ({ ...old, ...patch }));
		queueController.push(patch);
	}
	useEffect(() => {
		alive.current = true;
		return () => {
			alive.current = false;
			void queueController.flush();
		};
	}, [queueController]);
	const state = { ...device.state, ...draft };
	const disabled = !!device.error || !device.state;
	return (
		<article className="rounded-lg bg-neutral-900 p-3 space-y-4 border border-neutral-800">
			<div className="flex items-center gap-3">
				<Button
					aria-label={tr("a11y.toggle", { name: device.name })}
					title={
						device.error
							? `${tr("offline_help")}: ${renderMessage(locale, device.error)}`
							: tr("a11y.toggle", { name: device.name })
					}
					className="shrink-0"
					size="icon-sm"
					variant={device.state?.on ? "default" : "secondary"}
					disabled={disabled}
					onClick={() => queue({ on: state.on === 1 ? 0 : 1 })}
				>
					<Power />
				</Button>
				<span className="flex-1 truncate">{device.name}</span>
				<Button
					aria-label={tr("a11y.details", { name: device.name })}
					variant="ghost"
					className="shrink-0"
					size="icon-sm"
					onClick={onDetails}
				>
					<MoreHorizontal />
				</Button>
			</div>
			{disabled ? (
				<p className="text-red-300">{tr("offline_help")}</p>
			) : (
				<>
					<div className="flex items-center gap-3">
						<Thermometer size={16} />
						<TemperatureSlider
							aria-label={tr("a11y.temperature", { name: device.name })}
							min={143}
							max={344}
							value={[state.temperature ?? 143]}
							onValueChange={(v) => queue({ temperature: v[0] })}
						/>
						<span className="w-16 shrink-0 text-right tabular-nums">
							{Math.round(1_000_000 / (state.temperature ?? 143))} K
						</span>
					</div>
					<div className="flex items-center gap-3">
						<Sun size={16} />
						<Slider
							aria-label={tr("a11y.brightness", { name: device.name })}
							min={3}
							max={100}
							value={[state.brightness ?? 3]}
							onValueChange={(v) => queue({ brightness: v[0] })}
						/>
						<span className="w-16 shrink-0 text-right tabular-nums">
							{state.brightness}%
						</span>
					</div>
				</>
			)}
		</article>
	);
}
