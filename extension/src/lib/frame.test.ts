import { describe, expect, it } from 'vitest';
import { frameHeight, fromPage, MOST_HEIGHT } from './frame';

describe('the page in the popup', () => {
	it('is understood, and nothing else', () => {
		expect(fromPage({ type: 'ottie-popup-ready' })).toEqual({ type: 'ottie-popup-ready' });
		expect(fromPage({ type: 'ottie-popup-size', height: 300 })).toEqual({
			type: 'ottie-popup-size',
			height: 300
		});
		expect(fromPage({ type: 'ottie-popup-size', height: 'tall' })).toBeNull();
		expect(fromPage({ type: 'ottie-token', token: 'x' })).toBeNull();
		expect(fromPage('ottie-popup-ready')).toBeNull();
	});

	it('gets a frame as tall as it is, within the popup', () => {
		expect(frameHeight(300.2)).toBe(301);
		expect(frameHeight(5000)).toBe(MOST_HEIGHT);
		expect(frameHeight(0)).toBe(48);
	});
});
