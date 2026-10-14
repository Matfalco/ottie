import { describe, expect, it } from 'vitest';
import { canPreview, soundChoices, soundsChoices } from './sounds';

describe('notification sounds', () => {
	it("lists the system's, none, Ottie's sets, then one for each, in both languages", () => {
		expect(soundsChoices(['marimba', 'kelp'], 'en')).toEqual([
			{
				label: null,
				choices: [
					{ value: 'system', name: 'System default' },
					{ value: 'silent', name: 'Silent' }
				]
			},
			{
				label: 'Ottie’s sets',
				choices: [
					{ value: 'marimba', name: 'Marimba' },
					{ value: 'kelp', name: 'Kelp' }
				]
			},
			{ label: null, choices: [{ value: 'custom', name: 'Custom' }] }
		]);
		const french = soundsChoices(['holt'], 'fr');
		expect(french[1].choices).toEqual([{ value: 'holt', name: 'Catiche' }]);
		expect(french[2].choices[0].name).toBe('Personnalisé');
	});

	it("offers a kind's own sounds first when choosing each", () => {
		const failed = soundChoices(['log', 'tide'], 'failed', 'en');
		expect(failed.map((group) => group.label)).toEqual([
			null,
			'For a failed download',
			'For a finished download'
		]);
		expect(failed[1].choices).toEqual([
			{ value: 'log-failed', name: 'Log' },
			{ value: 'tide-failed', name: 'Tide' }
		]);
		expect(soundChoices(['log'], 'finished', 'fr')[1].choices).toEqual([
			{ value: 'log-finished', name: 'Rondin' }
		]);
	});

	it('shows the ID of a set it has no name for', () => {
		expect(soundsChoices(['otter'], 'en')[1].choices).toEqual([{ value: 'otter', name: 'otter' }]);
	});

	it("plays Ottie's sounds and sets, and the system's where the app can", () => {
		expect(canPreview('kelp', false)).toBe(true);
		expect(canPreview('kelp-failed', false)).toBe(true);
		expect(canPreview('system', true)).toBe(true);
		expect(canPreview('system', false)).toBe(false);
		expect(canPreview('silent', true)).toBe(false);
		expect(canPreview('custom', true)).toBe(false);
	});
});
