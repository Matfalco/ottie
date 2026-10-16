// How the page looks on this device: light, dark or whatever the system
// uses, and which pair of themes. Remembered by this browser; a device can
// also follow the look the daemon shares with every device that wants it
// (docs/decisions/0021, 0022, 0044, 0047).

import type { Appearance } from '@ottie/api-types/Appearance';
import type { AppearanceMode } from '@ottie/api-types/AppearanceMode';
import type { Palette } from '@ottie/api-types/Palette';

export type { Appearance, Palette };
export type Mode = AppearanceMode;
export type Theme = 'seafoam' | 'lagoon' | 'pearl' | 'slate';

/** The look on this device, and whether it follows the shared look and
 * language (yes unless switched off). */
export interface DeviceAppearance extends Appearance {
	synced: boolean;
}

export const MODES: Mode[] = ['system', 'light', 'dark'];
export const PALETTES: Palette[] = ['sea', 'neutral'];
export const DEFAULT_APPEARANCE: DeviceAppearance = {
	mode: 'system',
	palette: 'sea',
	synced: true
};

/** Each palette's light and dark theme. */
export const THEMES: Record<Palette, { light: Theme; dark: Theme }> = {
	sea: { light: 'seafoam', dark: 'lagoon' },
	neutral: { light: 'pearl', dark: 'slate' }
};

/** The legacy palettes there were (docs/decisions/0047): a device that
 * saved one gets its pair. */
const LEGACY: Record<string, Palette> = { seaLegacy: 'sea', neutralLegacy: 'neutral' };

const KEY = 'ottie.appearance';

/** The theme to show, given whether the system is in dark mode. */
export function themeFor(appearance: Appearance, systemDark: boolean): Theme {
	const dark = appearance.mode === 'dark' || (appearance.mode === 'system' && systemDark);
	return THEMES[appearance.palette][dark ? 'dark' : 'light'];
}

/** The themes a mode's preview shows, left half then right half: following
 * the system shows both. */
export function previewFor(mode: Mode, palette: Palette): [Theme, Theme] {
	const { light, dark } = THEMES[palette];
	return mode === 'system' ? [light, dark] : mode === 'light' ? [light, light] : [dark, dark];
}

/** Just the look, as the daemon shares it. */
export function lookOf({ mode, palette }: Appearance): Appearance {
	return { mode, palette };
}

export function sameLook(a: Appearance, b: Appearance): boolean {
	return a.mode === b.mode && a.palette === b.palette;
}

/** A stored appearance; what's missing or unknown falls back to the default. */
export function parseAppearance(text: string | null): DeviceAppearance {
	let stored: unknown = null;
	try {
		stored = text === null ? null : JSON.parse(text);
	} catch {
		// Not JSON: ignore it.
	}
	const fields = typeof stored === 'object' && stored !== null ? stored : {};
	const mode = 'mode' in fields ? fields.mode : undefined;
	const palette = 'palette' in fields ? fields.palette : undefined;
	const synced = 'synced' in fields ? fields.synced : undefined;
	return {
		mode: MODES.find((known) => known === mode) ?? DEFAULT_APPEARANCE.mode,
		palette:
			PALETTES.find((known) => known === palette) ??
			(typeof palette === 'string' ? LEGACY[palette] : undefined) ??
			DEFAULT_APPEARANCE.palette,
		synced: synced !== false
	};
}

export function loadAppearance(): DeviceAppearance {
	try {
		return parseAppearance(localStorage.getItem(KEY));
	} catch {
		return { ...DEFAULT_APPEARANCE }; // Storage blocked.
	}
}

/** Calls `changed` with the appearance another window of this page (a tab,
 * the desktop app's tray menu) saves. Returns what stops listening. */
export function followSavedAppearance(changed: (next: DeviceAppearance) => void): () => void {
	const saved = (event: StorageEvent) => {
		// `null`: the whole storage was cleared.
		if (event.key === KEY || event.key === null) changed(parseAppearance(event.newValue));
	};
	addEventListener('storage', saved);
	return () => removeEventListener('storage', saved);
}

export function saveAppearance(appearance: DeviceAppearance): void {
	try {
		localStorage.setItem(KEY, JSON.stringify(appearance));
	} catch {
		// Storage blocked: it lasts until the page is closed.
	}
}
