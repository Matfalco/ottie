// The toolbar's actions on the selected downloads: which of them each action
// applies to, and what to ask before one that deletes downloaded data.

import type { Job } from '@ottie/api-types/Job';
import { displayName, formatCount } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';
import { canCancel, canPause, canResume, removeLosesData } from './status';

export type Action = 'resume' | 'pause' | 'cancel' | 'remove';

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
