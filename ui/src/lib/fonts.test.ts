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
const faces = readFileSync(new URL('../fonts/faces.css', import.meta.url), 'utf8');

/** Each cut font's code points, by the locale before it in faces.css
 * (`/* ja *\/ @font-face { … unicode-range: U+20-7E, U+3001; }`). */
const COVERED = new Map(
	[...faces.matchAll(/\/\* (\S+) \*\/\s*@font-face \{[^}]*unicode-range:([^;]+);/g)].map(
		([, locale, list]) => {
			const ranges = list.split(',').map((range) => {
				const [from, to = from] = range.trim().replace('U+', '').split('-');
				return [parseInt(from, 16), parseInt(to, 16)] as const;
			});
			const covers = (point: number) => ranges.some(([from, to]) => from <= point && point <= to);
			return [locale, covers] as const;
		}
	)
);

/** Scripts Outfit and Manrope can't draw: Devanagari, CJK punctuation, kana,
 * Han, Hangul, full-width forms. */
const NEEDS_NOTO = /[ऀ-ॿ　-ヿ㐀-鿿가-힯＀-￯]/;

describe('fonts cut to the translations', () => {
	it('has one for each language whose script needs it', () => {
		const needing = LANGUAGES.filter(([, messages]) =>
			Object.values(messages).some((text) => NEEDS_NOTO.test(text))
		).map(([locale]) => locale);
		expect([...COVERED.keys()].sort()).toEqual(needing.sort());
	});

	for (const [locale, covers] of COVERED) {
		it(`holds every character of ${locale}’s messages and name (else: pnpm fonts)`, () => {
			const messages = LANGUAGES.find(([own]) => own === locale)?.[1] ?? {};
			const name = isLocale(locale) ? NAMES[locale] : '';
			const text = [...Object.values(messages), name].join('');
			const missing = [...new Set(text)].filter((c) => !covers(c.codePointAt(0) ?? 0));
			expect(missing).toEqual([]);
		});
	}
});
