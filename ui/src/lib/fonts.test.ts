import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { NAMES } from './languages';
import { isLocale } from './paraglide/runtime.js';

/** Every language's messages, by locale (`../../messages/fr.json` → `fr`). */
const FILES = import.meta.glob<Record<string, string>>('../../messages/*.json', {
	eager: true,
	import: 'default'
});
const LANGUAGES = Object.entries(FILES).map(
	([path, messages]) => [path.replace(/^.*\/|\.json$/g, ''), messages] as const
);

// (Read from disk: Vitest leaves CSS imports empty.)
const css = readFileSync(new URL('../app.css', import.meta.url), 'utf8');

/** The characters Outfit draws (@fontsource-variable/outfit 5.3.0, its
 * latin and latin-ext files), as hex code point ranges. */
const OUTFIT = (
	'20-7e a0-ac ae-107 10a-113 116-11b 11e-123 126-127 12a-12b 12e-133 136-137 139-13e ' +
	'141-148 14a-14d 150-15b 15e-161 164-165 16a-17e 1cd-1ce 218-21b 237 2bc 2c6-2c7 2da ' +
	'2dc-2dd 300-301 303-304 308 1e80-1e85 1e9e 1ef2-1ef3 2013-2014 2018-201a 201c-201e ' +
	'2022 2026 2039-203a 2044 20ac 2122 2191 2193 2212 2215'
)
	.split(' ')
	.map((range) => {
		const [from = '', to = from] = range.split('-');
		return [parseInt(from, 16), parseInt(to, 16)] as const;
	});

function inOutfit(c: string): boolean {
	const point = c.codePointAt(0) ?? 0;
	return OUTFIT.some(([from, to]) => from <= point && point <= to);
}

/** The languages app.css gives the system's font (`:root:lang(el), … {
 * --font: system-ui, …}`). */
function onSystemFont(): string[] {
	const rule = css.match(/((?::root:lang\([^)]+\),?\s*)+)\{\s*--font: system-ui/);
	return [...(rule?.[1] ?? '').matchAll(/:lang\(([^)]+)\)/g)].map(([, locale]) => locale ?? '');
}

describe('fonts', () => {
	it('gives the system’s font to the languages Outfit can’t fully draw', () => {
		// Letters and marks only: a lone symbol (→) or space from another font
		// doesn't break a word.
		const lacking = LANGUAGES.filter(([locale, messages]) => {
			const name = isLocale(locale) ? NAMES[locale] : '';
			const text = [...Object.values(messages), name].join('');
			return [...text].some((c) => /[\p{L}\p{M}]/u.test(c) && !inOutfit(c));
		}).map(([locale]) => locale);
		expect(onSystemFont().sort()).toEqual(lacking.sort());
	});
});
