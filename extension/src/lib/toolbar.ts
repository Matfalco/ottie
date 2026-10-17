// The button in the browser's toolbar shows the tray's otter (res/icons),
// black on a light toolbar and white on a dark one; the extensions page and
// the stores keep the app's icon. Firefox picks the shade from the manifest
// (`theme_icons`); Chromium-based browsers are told by the offscreen page,
// which follows `prefers-color-scheme` (docs/decisions/0055).

export type Shade = 'black' | 'white';

/** The sizes copied from res/icons/png/tray: 16 and 32 for the toolbar,
 * 24 for screens at 150 %. */
export const TOOLBAR_SIZES = [16, 24, 32] as const;

/** The toolbar's icons in one shade, as `action` takes them. */
export function toolbarIcons(shade: Shade): Record<number, string> {
	return Object.fromEntries(
		TOOLBAR_SIZES.map((size) => [size, `icons/toolbar-${shade}-${size}.png`])
	);
}

/** The shade for a toolbar in dark mode or not. */
export function shadeFor(dark: boolean): Shade {
	return dark ? 'white' : 'black';
}

/** Firefox's `theme_icons`: its "dark" one goes on themes with dark text
 * (a light toolbar), its "light" one on themes with light text. */
export function themeIcons(): { light: string; dark: string; size: number }[] {
	return TOOLBAR_SIZES.map((size) => ({
		light: `icons/toolbar-white-${size}.png`,
		dark: `icons/toolbar-black-${size}.png`,
		size
	}));
}

/** What the offscreen page sends the background: whether the browser is in
 * dark mode, at its start and at each change. */
export interface SchemeMessage {
	kind: 'ottie-scheme';
	dark: boolean;
}

export function isSchemeMessage(message: unknown): message is SchemeMessage {
	return (
		typeof message === 'object' &&
		message !== null &&
		(message as { kind?: unknown }).kind === 'ottie-scheme' &&
		typeof (message as { dark?: unknown }).dark === 'boolean'
	);
}
