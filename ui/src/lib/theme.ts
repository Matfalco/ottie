// How the page looks on this device: light, dark or whatever the system
// uses, and which pair of themes. Remembered by this browser
// (docs/decisions/0021).

export type Mode = 'system' | 'light' | 'dark';
export type Palette = 'sea' | 'neutral';
export type Theme = 'seafoam' | 'lagoon' | 'pearl' | 'slate';

export interface Appearance {
	mode: Mode;
	palette: Palette;
}

export const MODES: Mode[] = ['system', 'light', 'dark'];
export const PALETTES: Palette[] = ['sea', 'neutral'];
export const DEFAULT_APPEARANCE: Appearance = { mode: 'system', palette: 'sea' };

/** Each palette's light and dark theme. */
export const THEMES: Record<Palette, { light: Theme; dark: Theme }> = {
	sea: { light: 'seafoam', dark: 'lagoon' },
	neutral: { light: 'pearl', dark: 'slate' }
};

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

/** A stored appearance; what's missing or unknown falls back to the default. */
export function parseAppearance(text: string | null): Appearance {
	let stored: unknown = null;
	try {
		stored = text === null ? null : JSON.parse(text);
	} catch {
		// Not JSON: ignore it.
	}
	const fields = typeof stored === 'object' && stored !== null ? stored : {};
	const mode = 'mode' in fields ? fields.mode : undefined;
	const palette = 'palette' in fields ? fields.palette : undefined;
	return {
		mode: MODES.find((known) => known === mode) ?? DEFAULT_APPEARANCE.mode,
		palette: PALETTES.find((known) => known === palette) ?? DEFAULT_APPEARANCE.palette
	};
}

export function loadAppearance(): Appearance {
	try {
		return parseAppearance(localStorage.getItem(KEY));
	} catch {
		return DEFAULT_APPEARANCE; // Storage blocked.
	}
}

export function saveAppearance(appearance: Appearance): void {
	try {
		localStorage.setItem(KEY, JSON.stringify(appearance));
	} catch {
		// Storage blocked: it lasts until the page is closed.
	}
}
