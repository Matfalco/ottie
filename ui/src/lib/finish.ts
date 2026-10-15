// When everything has finished (docs/decisions/0040): what the desktop app
// does then, in words, what this computer can do, and the countdown before
// it sleeps, hibernates or shuts down.

import { formatDuration } from './format.js';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';
import type { Finish, Finishing, Power } from './window.js';

/** In the menus' order. */
export const FINISH_CHOICES: readonly Finish[] = [
	'nothing',
	'quit',
	'sleep',
	'hibernate',
	'shutDown'
];

/** The countdown's length people may choose, in seconds (as in `app/src/finish.rs`). */
export const COUNTDOWN = { min: 10, max: 600 } as const;

type Options = { locale: Locale };
type Text = (options: Options) => string;

const NAMES: Record<Finish, Text> = {
	nothing: (o) => m.finish_nothing({}, o),
	quit: (o) => m.finish_quit({}, o),
	sleep: (o) => m.finish_sleep({}, o),
	hibernate: (o) => m.finish_hibernate({}, o),
	shutDown: (o) => m.finish_shut_down({}, o)
};

const HINTS: Record<Finish, Text> = {
	nothing: (o) => m.finish_nothing_hint({}, o),
	quit: (o) => m.finish_quit_hint({}, o),
	sleep: (o) => m.finish_sleep_hint({}, o),
	hibernate: (o) => m.finish_hibernate_hint({}, o),
	shutDown: (o) => m.finish_shut_down_hint({}, o)
};

const CANT: Record<Power, Text> = {
	sleep: (o) => m.finish_cant_sleep({}, o),
	hibernate: (o) => m.finish_cant_hibernate({}, o),
	shutDown: (o) => m.finish_cant_shut_down({}, o)
};

const BUTTONS: Record<Finish, Text> = {
	nothing: (o) => m.finish_button({}, o),
	quit: (o) => m.finish_button_quit({}, o),
	sleep: (o) => m.finish_button_sleep({}, o),
	hibernate: (o) => m.finish_button_hibernate({}, o),
	shutDown: (o) => m.finish_button_shut_down({}, o)
};

const SOON: Record<Power, (time: string, options: Options) => string> = {
	sleep: (time, o) => m.finish_soon_sleep({ time }, o),
	hibernate: (time, o) => m.finish_soon_hibernate({ time }, o),
	shutDown: (time, o) => m.finish_soon_shut_down({ time }, o)
};

const NOW: Record<Power, Text> = {
	sleep: (o) => m.finish_now_sleep({}, o),
	hibernate: (o) => m.finish_now_hibernate({}, o),
	shutDown: (o) => m.finish_now_shut_down({}, o)
};

/** What happens next time: the choice for once, else every time's. */
export function nextFinish(finishing: Finishing): Finish {
	return finishing.once ?? finishing.always;
}

/** Whether this computer can do it; quitting or nothing it always can. */
export function canDo(finishing: Finishing, choice: Finish): boolean {
	return choice === 'nothing' || choice === 'quit' || finishing.able[choice];
}

/** "Shut down". */
export function finishName(choice: Finish, locale: Locale = getLocale()): string {
	return NAMES[choice]({ locale });
}

/** What it means, or why this computer can't. */
export function finishHint(
	finishing: Finishing,
	choice: Finish,
	locale: Locale = getLocale()
): string {
	if (choice !== 'nothing' && choice !== 'quit' && !finishing.able[choice]) {
		return CANT[choice]({ locale });
	}
	return HINTS[choice]({ locale });
}

/** The Downloads page's button: "When finished", "Shut down when finished". */
export function finishButton(choice: Finish, locale: Locale = getLocale()): string {
	return BUTTONS[choice]({ locale });
}

/** Whole seconds left before `endsAt`, rounded up: "1 sec" until the end. */
export function secondsLeft(endsAt: number, now: number): number {
	return Math.max(0, Math.ceil((endsAt - now) / 1000));
}

/** "Shut down in 45 sec". */
export function soonText(power: Power, seconds: number, locale: Locale = getLocale()): string {
	return SOON[power](formatDuration(seconds, locale), { locale });
}

/** "Shut down now". */
export function nowText(power: Power, locale: Locale = getLocale()): string {
	return NOW[power]({ locale });
}
