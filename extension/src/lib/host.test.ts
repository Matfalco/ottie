import { describe, expect, it } from 'vitest';
import { browserName, onThisComputer, parseAnswer } from './host';

describe('the host', () => {
	it('answers in a few shapes only', () => {
		expect(parseAnswer({ type: 'paired', address: 'http://127.0.0.1:24068', token: 't' })).toEqual({
			type: 'paired',
			address: 'http://127.0.0.1:24068',
			token: 't'
		});
		expect(parseAnswer({ type: 'started', address: 'a' })).toEqual({
			type: 'started',
			address: 'a'
		});
		expect(parseAnswer({ type: 'error', message: 'no' })).toEqual({
			type: 'error',
			message: 'no'
		});
		expect(parseAnswer({ type: 'paired', address: 'a' })).toBeNull();
		expect(parseAnswer('paired')).toBeNull();
	});

	it('names the browser', () => {
		const chrome =
			'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36';
		expect(browserName(chrome)).toBe('Chrome');
		expect(browserName(`${chrome} Edg/141.0.0.0`)).toBe('Microsoft Edge');
		expect(browserName(`${chrome} OPR/120.0.0.0`)).toBe('Opera');
		expect(browserName(chrome, true)).toBe('Brave');
		expect(
			browserName('Mozilla/5.0 (X11; Linux x86_64; rv:143.0) Gecko/20100101 Firefox/143.0')
		).toBe('Firefox');
		expect(browserName('Something')).toBe('Browser');
	});

	it('starts only Ottie on this computer', () => {
		expect(onThisComputer('http://127.0.0.1:24068')).toBe(true);
		expect(onThisComputer('http://localhost:24068')).toBe(true);
		expect(onThisComputer('http://[::1]:24068')).toBe(true);
		expect(onThisComputer('https://nas.example.ts.net')).toBe(false);
		expect(onThisComputer('nonsense')).toBe(false);
	});
});
