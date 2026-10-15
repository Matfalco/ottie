import { describe, expect, it } from 'vitest';
import {
	canDo,
	finishButton,
	finishHint,
	nextFinish,
	nowText,
	secondsLeft,
	soonText
} from './finish';
import { formatDuration } from './format';
import type { Finishing } from './window';

const finishing = (change: Partial<Finishing> = {}): Finishing => ({
	always: 'nothing',
	once: null,
	able: { sleep: true, hibernate: false, shutDown: true },
	countdown: 60,
	countdownIn: 'window',
	waitsForScheduled: true,
	running: null,
	...change
});

describe('when everything has finished', () => {
	it('does the choice for once, else every time’s', () => {
		expect(nextFinish(finishing({ always: 'shutDown' }))).toBe('shutDown');
		expect(nextFinish(finishing({ always: 'shutDown', once: 'nothing' }))).toBe('nothing');
	});

	it('says why this computer can’t', () => {
		const now = finishing();
		expect(canDo(now, 'quit')).toBe(true);
		expect(canDo(now, 'hibernate')).toBe(false);
		expect(finishHint(now, 'hibernate', 'en')).toBe('Hibernation is off on this computer');
		expect(finishHint(now, 'shutDown', 'en')).toBe('Turns the computer off');
	});

	it('names the button after what will happen, in both languages', () => {
		expect(finishButton('nothing', 'en')).toBe('When finished');
		expect(finishButton('shutDown', 'en')).toBe('Shut down when finished');
		expect(finishButton('sleep', 'fr')).toBe('Mettre en veille à la fin');
	});

	it('counts down in whole seconds, rounded up', () => {
		expect(secondsLeft(10_000, 9_001)).toBe(1);
		expect(secondsLeft(10_000, 10_500)).toBe(0);
		expect(secondsLeft(70_000, 10_000)).toBe(60);
		expect(soonText('shutDown', 45, 'en')).toBe('Shut down in 45 sec');
		expect(soonText('sleep', 90, 'fr')).toBe(`Mise en veille dans ${formatDuration(90, 'fr')}`);
		expect(nowText('hibernate', 'en')).toBe('Hibernate now');
	});
});
