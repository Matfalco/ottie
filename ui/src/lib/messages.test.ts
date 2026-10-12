import { describe, expect, it } from 'vitest';
import en from '../../messages/en.json';
import fr from '../../messages/fr.json';

const LANGUAGES: Record<string, Record<string, string>> = { fr };
const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe('translations', () => {
	for (const [locale, messages] of Object.entries(LANGUAGES)) {
		it(`${locale} has every English message, with the same placeholders`, () => {
			expect(Object.keys(messages).sort()).toEqual(Object.keys(en).sort());
			for (const [key, text] of Object.entries(en)) {
				expect(placeholders(messages[key]), key).toEqual(placeholders(text));
			}
		});
	}
});
