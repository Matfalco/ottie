import { describe, expect, it } from 'vitest';
import { DEFAULTS, parseSettings } from './settings';

describe('parseSettings', () => {
	it('gives the defaults for nothing stored', () => {
		expect(parseSettings({})).toEqual(DEFAULTS);
	});

	it('keeps what is valid and drops the rest', () => {
		expect(
			parseSettings({
				pairing: { address: 'http://127.0.0.1:24068', token: 't' },
				takeOver: false,
				minSize: 2048,
				excluded: ['example.com', 3]
			})
		).toEqual({
			pairing: { address: 'http://127.0.0.1:24068', token: 't' },
			takeOver: false,
			minSize: 2048,
			excluded: ['example.com']
		});
		expect(
			parseSettings({ pairing: { address: 1 }, takeOver: 'yes', minSize: -1, excluded: 'x' })
		).toEqual(DEFAULTS);
	});
});
