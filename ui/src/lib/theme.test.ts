import { describe, expect, it } from 'vitest';
import { DEFAULT_APPEARANCE, parseAppearance, previewFor, themeFor } from './theme';

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

describe('previewFor', () => {
	it('shows both themes of the palette when following the system', () => {
		expect(previewFor('system', 'sea')).toEqual(['seafoam', 'lagoon']);
		expect(previewFor('light', 'neutral')).toEqual(['pearl', 'pearl']);
		expect(previewFor('dark', 'neutral')).toEqual(['slate', 'slate']);
	});
});

describe('parseAppearance', () => {
	it('reads what was saved', () => {
		expect(parseAppearance('{"mode":"dark","palette":"neutral"}')).toEqual({
			mode: 'dark',
			palette: 'neutral'
		});
	});

	it('falls back to the default for anything missing or unknown', () => {
		expect(parseAppearance(null)).toEqual(DEFAULT_APPEARANCE);
		expect(parseAppearance('not json')).toEqual(DEFAULT_APPEARANCE);
		expect(parseAppearance('"dark"')).toEqual(DEFAULT_APPEARANCE);
		expect(parseAppearance('{"mode":"dim","palette":"neutral"}')).toEqual({
			mode: 'system',
			palette: 'neutral'
		});
	});
});
