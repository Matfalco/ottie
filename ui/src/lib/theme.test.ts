import { describe, expect, it } from 'vitest';
import {
	DEFAULT_APPEARANCE,
	lookOf,
	parseAppearance,
	previewFor,
	styleFor,
	themeFor,
	type DeviceAppearance
} from './theme';

describe('themeFor', () => {
	it('follows the system unless told otherwise', () => {
		expect(themeFor({ mode: 'system', palette: 'sea' }, false)).toBe('seafoam');
		expect(themeFor({ mode: 'system', palette: 'sea' }, true)).toBe('lagoon');
		expect(themeFor({ mode: 'system', palette: 'neutral' }, false)).toBe('pearl');
		expect(themeFor({ mode: 'system', palette: 'neutral' }, true)).toBe('slate');
	});

	it('keeps a chosen mode whatever the system does', () => {
		expect(themeFor({ mode: 'light', palette: 'sea' }, true)).toBe('seafoam');
		expect(themeFor({ mode: 'dark', palette: 'neutral' }, false)).toBe('slate');
	});
});

describe('styleFor', () => {
	it('draws the legacy palettes as before, the others rounded', () => {
		expect(styleFor('sea')).toBe('rounded');
		expect(styleFor('neutral')).toBe('rounded');
		expect(styleFor('seaLegacy')).toBe('legacy');
		expect(styleFor('neutralLegacy')).toBe('legacy');
	});

	it('keeps the colours of the palette a legacy one comes from', () => {
		expect(themeFor({ mode: 'dark', palette: 'seaLegacy' }, false)).toBe('lagoon');
		expect(themeFor({ mode: 'light', palette: 'neutralLegacy' }, true)).toBe('pearl');
	});
});

describe('previewFor', () => {
	it('shows both themes of the palette when following the system', () => {
		expect(previewFor('system', 'sea')).toEqual(['seafoam', 'lagoon']);
		expect(previewFor('light', 'neutral')).toEqual(['pearl', 'pearl']);
		expect(previewFor('dark', 'neutral')).toEqual(['slate', 'slate']);
	});
});

describe('parseAppearance', () => {
	it('reads what was saved', () => {
		expect(parseAppearance('{"mode":"dark","palette":"neutral","synced":false}')).toEqual({
			mode: 'dark',
			palette: 'neutral',
			synced: false
		});
		expect(parseAppearance('{"mode":"light","palette":"seaLegacy"}').palette).toBe('seaLegacy');
	});

	it('falls back to the default for anything missing or unknown', () => {
		expect(parseAppearance(null)).toEqual(DEFAULT_APPEARANCE);
		expect(parseAppearance('not json')).toEqual(DEFAULT_APPEARANCE);
		expect(parseAppearance('"dark"')).toEqual(DEFAULT_APPEARANCE);
		// Saved before the switch existed: it follows, like every new device.
		expect(parseAppearance('{"mode":"dim","palette":"neutral"}')).toEqual({
			mode: 'system',
			palette: 'neutral',
			synced: true
		});
	});
});

describe('lookOf', () => {
	it('leaves out what belongs to this device only', () => {
		const device: DeviceAppearance = { mode: 'dark', palette: 'sea', synced: true };
		expect(lookOf(device)).toEqual({
			mode: 'dark',
			palette: 'sea'
		});
	});
});
