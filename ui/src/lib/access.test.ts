import { describe, expect, it } from 'vitest';
import type { Device } from '@ottie/api-types/Device';
import { deviceLine, folderRows, withFolder } from './access';

const FOLDERS = { config: ['/srv/films'], chosen: ['/srv/music', '/srv/films'] };

describe('folders devices may use', () => {
	it('lists config.toml’s first, each folder once', () => {
		expect(folderRows(FOLDERS)).toEqual([
			{ folder: '/srv/films', fromConfig: true },
			{ folder: '/srv/music', fromConfig: false }
		]);
	});

	it('adds a folder once, trimmed', () => {
		expect(withFolder(FOLDERS, ' /srv/books ')).toEqual(['/srv/music', '/srv/films', '/srv/books']);
		expect(withFolder(FOLDERS, '/srv/music')).toBeNull();
		expect(withFolder(FOLDERS, '/srv/films')).toBeNull();
		expect(withFolder(FOLDERS, '  ')).toBeNull();
	});
});

describe('deviceLine', () => {
	const phone: Device = { id: 1, name: 'Phone', readOnly: true, created: 0, lastUsed: null };

	it('says what it may do and when it was last used', () => {
		expect(deviceLine(phone, 'en')).toBe('Read-only · Never used');
		expect(deviceLine({ ...phone, readOnly: false }, 'fr')).toBe('Accès complet · Jamais utilisé');
		const used = deviceLine({ ...phone, lastUsed: Date.UTC(2026, 9, 16, 12) }, 'en');
		expect(used).toMatch(/^Read-only · Last used .*2026/);
	});
});
