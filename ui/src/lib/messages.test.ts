import { describe, expect, it } from 'vitest';
import en from '../../messages/en.json';
import { NAMES } from './languages';
import { locales } from './paraglide/runtime.js';

/** Every language's messages, by locale (`../../messages/fr.json` → `fr`). */
const FILES = import.meta.glob<Record<string, string>>('../../messages/*.json', {
	eager: true,
	import: 'default'
});
const LANGUAGES = Object.fromEntries(
	Object.entries(FILES).map(([path, messages]) => [path.replace(/^.*\/|\.json$/g, ''), messages])
);
const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe('translations', () => {
	it('has a file and a name for every language, and nothing else', () => {
		expect(Object.keys(LANGUAGES).sort()).toEqual([...locales].sort());
		expect(Object.keys(NAMES).sort()).toEqual([...locales].sort());
	});

	for (const [locale, messages] of Object.entries(LANGUAGES)) {
		it(`${locale} has every English message, with the same placeholders`, () => {
			expect(Object.keys(messages).sort()).toEqual(Object.keys(en).sort());
			for (const [key, text] of Object.entries(en)) {
				expect(placeholders(messages[key]), key).toEqual(placeholders(text));
			}
		});
	}
});
