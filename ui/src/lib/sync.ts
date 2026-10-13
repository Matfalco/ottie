// Devices that follow the daemon's shared look and language (all of them,
// unless switched off): what to do when its settings arrive or change
// (docs/decisions/0022).

import type { Appearance } from '@ottie/api-types/Appearance';
import type { SettingsChange } from '@ottie/api-types/SettingsChange';
import { lookOf, sameLook, type DeviceAppearance } from './theme';

export interface Shared {
	appearance: Appearance | null;
	language: string | null;
}

export interface SyncSteps {
	/** Take on this look. */
	look: Appearance | null;
	/** Switch to this language (one this page has). */
	language: string | null;
	/** Share what this device has, where nothing is shared yet. */
	share: SettingsChange | null;
}

/** For a device showing `device` in `locale`, and able to show `known`
 * languages. A shared language it doesn't have leaves it in its own. */
export function syncSteps(
	device: DeviceAppearance,
	locale: string,
	shared: Shared,
	known: readonly string[]
): SyncSteps {
	const steps: SyncSteps = { look: null, language: null, share: null };
	if (!device.synced) {
		return steps;
	}
	const share: SettingsChange = {};
	if (shared.appearance === null) {
		share.appearance = lookOf(device);
	} else if (!sameLook(device, shared.appearance)) {
		steps.look = lookOf(shared.appearance);
	}
	if (shared.language === null) {
		share.language = locale;
	} else if (shared.language !== locale && known.includes(shared.language)) {
		steps.language = shared.language;
	}
	steps.share = Object.keys(share).length > 0 ? share : null;
	return steps;
}
