import { describe, expect, it } from 'vitest';
import { syncSteps } from './sync';
import type { DeviceAppearance } from './theme';

const KNOWN = ['en', 'fr'];
const device: DeviceAppearance = { mode: 'dark', palette: 'sea', synced: true };
const look = { mode: 'dark', palette: 'sea' } as const;

describe('syncSteps', () => {
	it('shares its look while none is shared', () => {
		expect(syncSteps(device, null, { appearance: null, language: null }, KNOWN)).toEqual({
			look: null,
			language: null,
			systemLanguage: false,
			share: { appearance: { mode: 'dark', palette: 'sea' } }
		});
	});

	it('takes on what is shared', () => {
		const shared = { appearance: { mode: 'light', palette: 'neutral' } as const, language: 'en' };
		expect(syncSteps(device, 'fr', shared, KNOWN)).toEqual({
			look: { mode: 'light', palette: 'neutral' },
			language: 'en',
			systemLanguage: false,
			share: null
		});
	});

	it('chooses the shared language even when the system’s is the same', () => {
		const shared = { appearance: look, language: 'fr' };
		expect(syncSteps(device, null, shared, KNOWN).language).toBe('fr');
	});

	it('goes back to the system’s language when that is what’s shared', () => {
		const shared = { appearance: look, language: null };
		expect(syncSteps(device, 'fr', shared, KNOWN).systemLanguage).toBe(true);
		expect(syncSteps(device, null, shared, KNOWN).systemLanguage).toBe(false);
	});

	it('does nothing once they agree', () => {
		const shared = { appearance: look, language: 'fr' };
		expect(syncSteps(device, 'fr', shared, KNOWN)).toEqual({
			look: null,
			language: null,
			systemLanguage: false,
			share: null
		});
	});

	it('keeps its own language when it doesn’t have the shared one', () => {
		const shared = { appearance: look, language: 'de' };
		expect(syncSteps(device, 'fr', shared, KNOWN).language).toBeNull();
	});

	it('leaves a device that doesn’t follow alone', () => {
		const own = { ...device, synced: false };
		expect(syncSteps(own, 'fr', { appearance: null, language: 'en' }, KNOWN)).toEqual({
			look: null,
			language: null,
			systemLanguage: false,
			share: null
		});
		expect(syncSteps(own, 'fr', { appearance: look, language: null }, KNOWN).systemLanguage).toBe(
			false
		);
	});
});
