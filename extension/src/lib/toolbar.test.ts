import { describe, expect, it } from 'vitest';
import { isSchemeMessage, shadeFor, themeIcons, toolbarIcons } from './toolbar';

describe('toolbar icons', () => {
	it('names each size in one shade', () => {
		expect(toolbarIcons('white')).toEqual({
			16: 'icons/toolbar-white-16.png',
			24: 'icons/toolbar-white-24.png',
			32: 'icons/toolbar-white-32.png'
		});
	});

	it('goes white on a dark toolbar, black on a light one', () => {
		expect(shadeFor(true)).toBe('white');
		expect(shadeFor(false)).toBe('black');
		expect(themeIcons()[0]).toEqual({
			light: 'icons/toolbar-white-16.png',
			dark: 'icons/toolbar-black-16.png',
			size: 16
		});
	});

	it('takes only the offscreen page’s messages', () => {
		expect(isSchemeMessage({ kind: 'ottie-scheme', dark: true })).toBe(true);
		expect(isSchemeMessage({ kind: 'ottie-scheme', dark: 'yes' })).toBe(false);
		expect(isSchemeMessage({ kind: 'other', dark: true })).toBe(false);
		expect(isSchemeMessage(null)).toBe(false);
	});
});
