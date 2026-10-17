import { describe, expect, it } from 'vitest';
import { DEFAULTS, type ExtensionSettings } from './settings';
import { skipReason, type BrowserDownload } from './takeover';

const SIGNED_IN: ExtensionSettings = {
	...DEFAULTS,
	pairing: { address: 'http://127.0.0.1:24068', token: 'abc' }
};

const ITEM: BrowserDownload = {
	url: 'https://example.com/files/a.zip',
	incognito: false,
	state: 'in_progress',
	totalBytes: 5_000_000
};

const never = () => false;

describe('skipReason', () => {
	it('takes an ordinary download', () => {
		expect(skipReason(ITEM, SIGNED_IN, never)).toBeNull();
	});

	it('takes one of unknown size, whatever the smallest size', () => {
		const settings = { ...SIGNED_IN, minSize: 1024 };
		expect(skipReason({ ...ITEM, totalBytes: 0 }, settings, never)).toBeNull();
		expect(skipReason({ ...ITEM, totalBytes: -1 }, settings, never)).toBeNull();
	});

	it('leaves the rest to the browser, saying why', () => {
		const cases: [Partial<BrowserDownload>, Partial<ExtensionSettings>, string][] = [
			[{}, { takeOver: false }, 'off'],
			[{}, { pairing: null }, 'signedOut'],
			[{ incognito: true }, {}, 'private'],
			[{ url: 'blob:https://example.com/1234' }, {}, 'scheme'],
			[{ url: 'data:text/plain,hi' }, {}, 'scheme'],
			[{ url: 'not a link' }, {}, 'scheme'],
			[{ byExtensionId: 'other' }, {}, 'extension'],
			[{ state: 'complete' }, {}, 'notRunning'],
			[{ url: 'http://127.0.0.1:24068/api/v1/stream/x/a.mp4' }, {}, 'ottie'],
			[{ totalBytes: 1000 }, { minSize: 1024 }, 'small'],
			[{ url: 'https://dl.example.com/a.zip' }, { excluded: ['example.com'] }, 'excluded']
		];
		for (const [item, settings, why] of cases) {
			expect(skipReason({ ...ITEM, ...item }, { ...SIGNED_IN, ...settings }, never)).toBe(why);
		}
	});

	it('leaves a download that came from a form to the browser', () => {
		const posted = (url: string) => url === ITEM.url;
		expect(skipReason(ITEM, SIGNED_IN, posted)).toBe('posted');
	});
});
