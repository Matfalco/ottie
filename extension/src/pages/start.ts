// What every extension page does first: Ottie's fonts and themes, its
// language (shared by Ottie, else the browser's), then the page.

import '@fontsource-variable/outfit';
import '@fontsource/dm-mono/400.css';
import '@fontsource/dm-mono/500.css';
import '../../../ui/src/app.css';
import { mount, type Component } from 'svelte';
import { direction } from '#ui/languages.js';
import { overwriteGetLocale } from '#ui/paraglide/runtime.js';
import { DEFAULT_APPEARANCE, themeFor } from '#ui/theme.js';
import { loadLook, pageLanguage } from '../lib/look';

export async function start(page: Component, title: () => string): Promise<void> {
	const look = await loadLook();
	const shown = pageLanguage(look, navigator.languages);
	overwriteGetLocale(() => shown);
	const root = document.documentElement;
	root.lang = shown;
	root.dir = direction(shown);
	document.title = title();

	const dark = matchMedia('(prefers-color-scheme: dark)');
	const theme = () => {
		root.dataset.theme = themeFor(look.appearance ?? DEFAULT_APPEARANCE, dark.matches);
	};
	theme();
	dark.addEventListener('change', theme);

	const target = document.getElementById('app');
	if (target !== null) {
		mount(page, { target });
	}
}
