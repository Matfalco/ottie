import { describe, expect, it } from 'vitest';
import { pageLanguage } from './look';

describe('pageLanguage', () => {
	it('shows Ottie’s shared language first', () => {
		expect(pageLanguage({ language: 'fr', appearance: null }, ['de-DE'])).toBe('fr');
	});

	it('else the browser’s closest, else English', () => {
		const none = { language: null, appearance: null };
		expect(pageLanguage(none, ['pt-BR', 'en'])).toBe('pt-BR');
		expect(pageLanguage(none, ['de-AT'])).toBe('de');
		expect(pageLanguage(none, ['xx'])).toBe('en');
		expect(pageLanguage({ language: 'xx', appearance: null }, ['it'])).toBe('it');
	});
});
