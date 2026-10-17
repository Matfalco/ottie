// The extension's manifest (Manifest V3), one per browser: Chromium-based
// browsers run the background as a service worker, Firefox as a page's
// script, and only Chrome can hide its own download UI (docs/decisions/0055).

export type Browser = 'chrome' | 'firefox';

/** Firefox's ID for the extension: fixed, so the native host can name it. */
export const GECKO_ID = 'ottie@matfalco.github.io';

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
		action: { default_title: 'Ottie', default_icon: ICONS },
		options_ui: { page: 'options.html', open_in_tab: true },
		// Every site: a download's cookies, and wherever Ottie runs.
		host_permissions: ['<all_urls>']
	};
	const permissions = ['downloads', 'cookies', 'storage', 'webRequest'];
	if (browser === 'chrome') {
		return {
			...common,
			background: { service_worker: 'background.js', type: 'module' },
			permissions: [...permissions, 'downloads.ui'],
			minimum_chrome_version: '116'
		};
	}
	return {
		...common,
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
