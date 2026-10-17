// The extension's manifest (Manifest V3), one per browser: Chromium-based
// browsers run the background as a service worker, Firefox as a page's
// script, and only Chrome can hide its own download UI (docs/decisions/0055).

import { themeIcons, toolbarIcons } from './lib/toolbar';

export type Browser = 'chrome' | 'firefox';

/** Firefox's ID for the extension: fixed, so the native host can name it. */
export const GECKO_ID = 'ottie@matfalco.github.io';

/** The public half of the key that fixes the Chrome ID; the stores give
 * theirs when it's published there. */
const CHROME_KEY =
	'MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAz2Z65jAwG8UxM0pQLqF/C6lN0KOquf1BTa4ODilTg10qPhRHjePAN2KGy/v7g18yU8bH22zhXwKWV5lnJLH+/f1F/yLgAH6gqgJ90YRi/p735tXRVXoyk+MmPzpfKdo315ZAQe7PcR2W9h6FcGPOCxtuW7w1kD3K4ZhDUyW8/VVwLw9WK3RuS8wKS7Iw/J+RlkylFfb86elByWVrh5LrCENOtGo7gNFlUkog1G98dcsQecI34lPjf4Ghn4TPqTm1ROnTl4pWgL36S9ni4qY/wf/QXs5VApGHAyPLWaX2BXsCYG6Xs7WyRkkkoIhmx9s9Ih/04GkABNso/NwJXwcxswIDAQAB';

/** The app's icon: the extensions page and the stores. The toolbar has its own. */
const ICONS = { 16: 'icons/16.png', 32: 'icons/32.png', 48: 'icons/48.png', 128: 'icons/128.png' };

export function manifest(browser: Browser, version: string): Record<string, unknown> {
	const common = {
		manifest_version: 3,
		name: 'Ottie – Download Manager',
		short_name: 'Ottie',
		version,
		description:
			'Sends your downloads to Ottie: pause and resume, several connections, speed limits.',
		icons: ICONS,
		action: {
			default_title: 'Ottie',
			default_icon: toolbarIcons('black'),
			default_popup: 'popup.html'
		},
		options_ui: { page: 'options.html', open_in_tab: true },
		// Every site: a download's cookies, and wherever Ottie runs.
		host_permissions: ['<all_urls>']
	};
	const permissions = [
		'downloads',
		'cookies',
		'storage',
		'webRequest',
		'nativeMessaging',
		'contextMenus',
		'scripting'
	];
	if (browser === 'chrome') {
		return {
			...common,
			// Fixes its ID when loaded unpacked (njjhemepabjjkdpcdplfcgjiooggdpol),
			// which the native host's manifest names (`crates/ottie/src/extension.rs`).
			key: CHROME_KEY,
			background: { service_worker: 'background.js', type: 'module' },
			// The offscreen page tells the toolbar icon's shade (lib/toolbar.ts).
			permissions: [...permissions, 'downloads.ui', 'offscreen'],
			minimum_chrome_version: '116'
		};
	}
	return {
		...common,
		action: { ...common.action, theme_icons: themeIcons() },
		background: { scripts: ['background.js'], type: 'module' },
		permissions,
		browser_specific_settings: {
			gecko: {
				id: GECKO_ID,
				strict_min_version: '128.0',
				// Links and cookies go only to the person's own Ottie.
				data_collection_permissions: { required: ['none'] }
			}
		}
	};
}
