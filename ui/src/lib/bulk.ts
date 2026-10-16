// The toolbar's actions on the selected downloads: which of them each action
// applies to, and what to ask before one that deletes downloaded data. Also
// what "Pause all" and "Resume all" act on (the toolbar, the tray menu).

import type { Job } from '@ottie/api-types/Job';
import { displayName, formatCount } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';
import { canCancel, canPause, canResume, removeLosesData } from './status';

export type Action = 'resume' | 'pause' | 'cancel' | 'remove';

const RUNNING: Job['state'][] = ['connecting', 'downloading', 'retrying'];

const APPLIES: Record<Action, (job: Job) => boolean> = {
	resume: canResume,
	pause: canPause,
	cancel: canCancel,
	remove: () => true
};

/** The jobs the action applies to; the others are left alone. */
export function targets(jobs: Job[], action: Action): Job[] {
	return jobs.filter(APPLIES[action]);
}

/** What to ask before doing it, or null when nothing downloaded would be lost. */
export function confirmation(
	action: Action,
	jobs: Job[],
	locale: Locale = getLocale()
): string | null {
	if ((action !== 'cancel' && action !== 'remove') || !jobs.some(removeLosesData)) {
		return null;
	}
	const options = { locale };
	const [only] = jobs;
	if (jobs.length === 1 && only) {
		const name = displayName(only);
		return action === 'cancel'
			? m.cancel_confirm({ name }, options)
			: m.remove_confirm({ name }, options);
	}
	const count = formatCount(jobs.length, locale);
	return action === 'cancel'
		? m.cancel_confirm_many({ count }, options)
		: m.remove_confirm_many({ count }, options);
}

/** "Pause all", in two steps: the waiting downloads first, so pausing a
 * running one doesn't start one of them, then the running ones. */
export function pauseSteps(jobs: Job[]): { waiting: string[]; running: string[] } {
	const ids = (states: Job['state'][]) =>
		jobs.filter((job) => !job.moving && states.includes(job.state)).map((job) => job.id);
	return { waiting: ids(['queued']), running: ids(RUNNING) };
}

/** "Resume all": the paused downloads. Failed ones need a look first. */
export function resumeTargets(jobs: Job[]): string[] {
	return jobs.filter((job) => !job.moving && job.state === 'paused').map((job) => job.id);
}
