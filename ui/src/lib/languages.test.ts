import { describe, expect, it } from 'vitest';
import { closestLanguage } from './languages';

const KNOWN = ['en', 'fr', 'de', 'es-ES', 'es-419', 'pt-PT', 'pt-BR', 'zh-Hans', 'zh-Hant'];
const closest = (...wanted: string[]) => closestLanguage(wanted, KNOWN);

describe('closestLanguage', () => {
	it('takes a language the page has, or its bare language', () => {
		expect(closest('fr')).toBe('fr');
		expect(closest('fr-CA')).toBe('fr');
		expect(closest('de_CH')).toBe('de');
	});

	it('follows the system’s order of preference', () => {
		expect(closest('ja', 'de-AT', 'fr')).toBe('de');
		expect(closest('ja', 'ko')).toBeUndefined();
	});

	it('gives Latin America its Spanish, and Spain its own', () => {
		expect(closest('es-MX')).toBe('es-419');
		expect(closest('es-419')).toBe('es-419');
		expect(closest('es-ar')).toBe('es-419');
		expect(closest('es')).toBe('es-ES');
		expect(closest('es-ES')).toBe('es-ES');
		expect(closestLanguage(['es-MX'], ['en', 'es-ES'])).toBe('es-ES');
	});

	it('gives Brazil its Portuguese, and every other country Portugal’s', () => {
		expect(closest('pt-BR')).toBe('pt-BR');
		expect(closest('pt')).toBe('pt-PT');
		expect(closest('pt-AO')).toBe('pt-PT');
		expect(closestLanguage(['pt-BR'], ['en', 'pt-PT'])).toBe('pt-PT');
	});

	it('tells traditional Chinese from simplified', () => {
		expect(closest('zh-TW')).toBe('zh-Hant');
		expect(closest('zh-HK')).toBe('zh-Hant');
		expect(closest('zh-Hant-SG')).toBe('zh-Hant');
		expect(closest('zh-CN')).toBe('zh-Hans');
		expect(closest('zh')).toBe('zh-Hans');
	});
});
