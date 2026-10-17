// The extension's own settings, in the browser's storage for extensions (on
// this device only): where Ottie is and the token it gave this browser, and
// which downloads it takes over (docs/decisions/0055).

/** Where Ottie answers (`http://127.0.0.1:24068`, no path) and this
 * browser's token there. */
export interface Pairing {
	address: string;
	token: string;
}

export interface ExtensionSettings {
	/** null: not signed in; every download stays in the browser. */
	pairing: Pairing | null;
	/** Hand downloads over to Ottie at all. */
	takeOver: boolean;
	/** Files the browser says are smaller (in bytes) stay there; 0: none do. */
	minSize: number;
	/** Sites (host names) whose downloads stay in the browser; each covers
	 * its subdomains. */
	excluded: string[];
}

export const DEFAULTS: ExtensionSettings = {
	pairing: null,
	takeOver: true,
	minSize: 0,
	excluded: []
};

/** Settings from what storage holds, whatever an older or damaged copy has. */
export function parseSettings(stored: Record<string, unknown>): ExtensionSettings {
	const pairing = stored.pairing as Partial<Pairing> | null | undefined;
	const minSize = stored.minSize;
	const excluded = stored.excluded;
	return {
		pairing:
			typeof pairing?.address === 'string' && typeof pairing.token === 'string'
				? { address: pairing.address, token: pairing.token }
				: null,
		takeOver: typeof stored.takeOver === 'boolean' ? stored.takeOver : DEFAULTS.takeOver,
		minSize:
			typeof minSize === 'number' && Number.isFinite(minSize) && minSize >= 0
				? minSize
				: DEFAULTS.minSize,
		excluded: Array.isArray(excluded)
			? excluded.filter((site): site is string => typeof site === 'string')
			: DEFAULTS.excluded
	};
}

const KEYS = Object.keys(DEFAULTS);

export async function loadSettings(): Promise<ExtensionSettings> {
	return parseSettings(await chrome.storage.local.get(KEYS));
}

export async function saveSettings(change: Partial<ExtensionSettings>): Promise<void> {
	await chrome.storage.local.set(change);
}

/** Calls `changed` with the new settings whenever any page or the background
 * changes them. Returns what stops listening. */
export function followSettings(changed: (settings: ExtensionSettings) => void): () => void {
	const listener = (_: unknown, area: string) => {
		if (area === 'local') {
			void loadSettings().then(changed);
		}
	};
	chrome.storage.onChanged.addListener(listener);
	return () => chrome.storage.onChanged.removeListener(listener);
}
