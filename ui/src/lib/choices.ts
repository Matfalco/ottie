// The choices both Settings and the first-start setup offer, in the order
// shown, with their names.

import type { Existing } from '@ottie/api-types/Existing';
import { m } from '#lib/paraglide/messages.js';
import type { Copied, Notify } from './window.js';

/** What a download does with a file already there (docs/decisions/0049). */
export const EXISTING_CHOICES: Existing[] = ['rename', 'replace', 'ask'];
export const EXISTING_NAMES: Record<Existing, () => string> = {
	rename: m.set_existing_rename,
	replace: m.set_existing_replace,
	ask: m.set_existing_ask
};

/** When finished downloads are notified (docs/decisions/0035). */
export const NOTIFY_NAMES: Record<Notify, { name: () => string; hint: () => string }> = {
	all: { name: m.set_notify_all, hint: m.set_notify_all_hint },
	each: { name: m.set_notify_each, hint: m.set_notify_each_hint },
	never: { name: m.set_notify_never, hint: m.set_notify_never_hint }
};
export const NOTIFY_CHOICES = Object.keys(NOTIFY_NAMES) as Notify[];

/** What a link copied elsewhere does (docs/decisions/0043). */
export const COPIED_NAMES: Record<Copied, { name: () => string; hint: () => string }> = {
	ask: { name: m.set_copied_ask, hint: m.set_copied_ask_hint },
	add: { name: m.set_copied_add, hint: m.set_copied_add_hint }
};
export const COPIED_CHOICES = Object.keys(COPIED_NAMES) as Copied[];
