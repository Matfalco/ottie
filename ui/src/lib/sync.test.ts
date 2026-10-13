import { describe, expect, it } from 'vitest';
import { syncSteps } from './sync';
import type { DeviceAppearance } from './theme';

const KNOWN = ['en', 'fr'];
const device: DeviceAppearance = { mode: 'dark', palette: 'sea', synced: true };

describe('syncSteps', () => {
	it('shares what this device has while nothing is shared', () => {
		expect(syncSteps(device, 'fr', { appearance: null, language: null }, KNOWN)).toEqual({
			look: null,
			language: null,
			share: { appearance: { mode: 'dark', palette: 'sea' }, language: 'fr' }
		});
	});

	it('takes on what is shared', () => {
		const shared = { appearance: { mode: 'light', palette: 'neutral' } as const, language: 'en' };
		expect(syncSteps(device, 'fr', shared, KNOWN)).toEqual({
			look: { mode: 'light', palette: 'neutral' },
			language: 'en',
			share: null
		});
	});

	it('does nothing once they agree', () => {
		const shared = { appearance: { mode: 'dark', palette: 'sea' } as const, language: 'fr' };
		expect(syncSteps(device, 'fr', shared, KNOWN)).toEqual({
			look: null,
			language: null,
			share: null
		});
	});

	it('keeps its own language when it doesn’t have the shared one', () => {
		const shared = { appearance: { mode: 'dark', palette: 'sea' } as const, language: 'de' };
		expect(syncSteps(device, 'fr', shared, KNOWN).language).toBeNull();
	});

	it('leaves a device that doesn’t follow alone', () => {
		const own = { ...device, synced: false };
		expect(syncSteps(own, 'fr', { appearance: null, language: 'en' }, KNOWN)).toEqual({
			look: null,
			language: null,
			share: null
		});
	});
});
