import { describe, expect, it } from 'vitest';
import { GECKO_ID, manifest } from './manifest';

describe('manifest', () => {
	it('runs the background as each browser can', () => {
		expect(manifest('chrome', '1.2.3').background).toEqual({
			service_worker: 'background.js',
			type: 'module'
		});
		expect(manifest('firefox', '1.2.3').background).toEqual({
			scripts: ['background.js'],
			type: 'module'
		});
	});

	it('fixes its ID in Chromium-based browsers', async () => {
		const key = manifest('chrome', '1').key as string;
		const digest = await crypto.subtle.digest('SHA-256', Buffer.from(key, 'base64'));
		const id = [...new Uint8Array(digest).slice(0, 16)]
			.flatMap((byte) => [byte >> 4, byte & 15])
			.map((nibble) => String.fromCharCode(97 + nibble))
			.join('');
		expect(id).toBe('njjhemepabjjkdpcdplfcgjiooggdpol');
	});

	it('keeps the app’s icon for the extensions page, the tray’s for the toolbar', () => {
		for (const browser of ['chrome', 'firefox'] as const) {
			const made = manifest(browser, '1');
			const action = made.action as { default_icon: Record<number, string> };
			expect((made.icons as Record<number, string>)[128]).toBe('icons/128.png');
			expect(action.default_icon[16]).toBe('icons/toolbar-black-16.png');
		}
		expect(manifest('chrome', '1').permissions).toContain('offscreen');
		expect(manifest('firefox', '1').permissions).not.toContain('offscreen');
		expect(manifest('firefox', '1').action).toHaveProperty('theme_icons');
	});

	it('asks for the download UI only where it exists, and names Firefox’s ID', () => {
		expect(manifest('chrome', '1').permissions).toContain('downloads.ui');
		expect(manifest('firefox', '1').permissions).not.toContain('downloads.ui');
		const gecko = (manifest('firefox', '1').browser_specific_settings as { gecko: { id: string } })
			.gecko;
		expect(gecko.id).toBe(GECKO_ID);
	});
});
