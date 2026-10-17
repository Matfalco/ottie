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

	it('asks for the download UI only where it exists, and names Firefox’s ID', () => {
		expect(manifest('chrome', '1').permissions).toContain('downloads.ui');
		expect(manifest('firefox', '1').permissions).not.toContain('downloads.ui');
		const gecko = (manifest('firefox', '1').browser_specific_settings as { gecko: { id: string } })
			.gecko;
		expect(gecko.id).toBe(GECKO_ID);
	});
});
