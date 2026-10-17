// Chromium-based browsers' hidden page: a service worker can't see whether
// the browser is in dark mode, so this tells the background, at its start
// and at each change, which shade of the toolbar icon to show (lib/toolbar.ts).

import type { SchemeMessage } from '../lib/toolbar';

const dark = matchMedia('(prefers-color-scheme: dark)');

function tell(): void {
	const message: SchemeMessage = { kind: 'ottie-scheme', dark: dark.matches };
	void chrome.runtime.sendMessage(message).catch(() => undefined);
}

tell();
dark.addEventListener('change', tell);
