/** Serializes writes and retains only the newest pending value of each property. */
export function createCoalescer<T extends object>(
	send: (patch: T) => Promise<void>,
	settled: () => void,
	failed: (error: unknown) => void,
	delay = 100,
) {
	let pending: Partial<T> = {};
	let running = false;
	let timer: ReturnType<typeof setTimeout> | undefined;
	async function flush(): Promise<void> {
		clearTimeout(timer);
		if (running) return;
		running = true;
		try {
			while (Object.keys(pending).length) {
				const patch = pending as T;
				pending = {};
				try {
					await send(patch);
				} catch (error) {
					failed(error);
				}
			}
		} finally {
			running = false;
			settled();
		}
	}
	return {
		push(patch: T) {
			pending = { ...pending, ...patch };
			clearTimeout(timer);
			timer = setTimeout(() => void flush(), delay);
		},
		flush,
	};
}
