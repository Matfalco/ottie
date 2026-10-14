// The desktop app's notification sounds (docs/decisions/0038): the system's,
// none, one of Ottie's sets (a sound for a finished download and one for a
// failed one), or a sound chosen for each. The app lists the sets by ID.

import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** The sounds chosen: `system`, `silent`, a set's ID or `custom`. */
export type Sounds = string;

/** One sound: `system`, `silent` or a set's (`kelp-failed`). */
export type Sound = string;

/** Which notifications a sound is for. */
export type SoundKind = 'finished' | 'failed';

/** Ottie's sets by ID. One the page doesn't know (a newer app) shows its ID. */
const SET_NAMES: Record<string, (locale: Locale) => string> = {
	marimba: (locale) => m.sound_set_marimba({}, { locale }),
	log: (locale) => m.sound_set_log({}, { locale }),
	driftwood: (locale) => m.sound_set_driftwood({}, { locale }),
	shell: (locale) => m.sound_set_shell({}, { locale }),
	stream: (locale) => m.sound_set_stream({}, { locale }),
	splash: (locale) => m.sound_set_splash({}, { locale }),
	burrow: (locale) => m.sound_set_burrow({}, { locale }),
	tide: (locale) => m.sound_set_tide({}, { locale }),
	holt: (locale) => m.sound_set_holt({}, { locale }),
	kelp: (locale) => m.sound_set_kelp({}, { locale })
};

export interface Choice {
	value: string;
	name: string;
}

/** Choices shown together: under a heading, or none (`label` null). */
export interface ChoiceGroup {
	label: string | null;
	choices: Choice[];
}

function setName(id: string, locale: Locale): string {
	return SET_NAMES[id]?.(locale) ?? id;
}

function plain(locale: Locale): ChoiceGroup {
	const options = { locale };
	return {
		label: null,
		choices: [
			{ value: 'system', name: m.sound_system({}, options) },
			{ value: 'silent', name: m.sound_silent({}, options) }
		]
	};
}

/** The sounds to choose from: the system's, none, Ottie's sets, then a
 * sound for each kind (`sets`: the app's, in its order). */
export function soundsChoices(sets: string[], locale: Locale = getLocale()): ChoiceGroup[] {
	const options = { locale };
	return [
		plain(locale),
		{
			label: m.sound_group_sets({}, options),
			choices: sets.map((id) => ({ value: id, name: setName(id, locale) }))
		},
		{ label: null, choices: [{ value: 'custom', name: m.sound_custom({}, options) }] }
	];
}

/** One kind's sound, when chosen for each: the system's, none, then the
 * sets' sounds for this kind, then those for the other one. */
export function soundChoices(
	sets: string[],
	kind: SoundKind,
	locale: Locale = getLocale()
): ChoiceGroup[] {
	const options = { locale };
	const group = (of: SoundKind): ChoiceGroup => ({
		label:
			of === 'finished' ? m.sound_group_finished({}, options) : m.sound_group_failed({}, options),
		choices: sets.map((id) => ({ value: `${id}-${of}`, name: setName(id, locale) }))
	});
	const other: SoundKind = kind === 'finished' ? 'failed' : 'finished';
	return [plain(locale), group(kind), group(other)];
}

/** Whether the page can play it to hear it: Ottie's sounds and sets, and
 * the system's where the app can play it (`systemSound`: on Windows). */
export function canPreview(value: string, systemSound: boolean): boolean {
	if (value === 'system') {
		return systemSound;
	}
	return value !== 'silent' && value !== 'custom';
}
