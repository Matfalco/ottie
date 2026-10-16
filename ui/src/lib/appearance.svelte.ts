// The appearance chosen on this device, shared by the layout (which applies
// it), the shell (which follows the shared one) and the Settings page (which
// changes it).

import {
	followSavedAppearance,
	loadAppearance,
	saveAppearance,
	type DeviceAppearance
} from './theme';

/** Change it only through `setAppearance`, so the choice is remembered. */
export const appearance: DeviceAppearance = $state(loadAppearance());

export function setAppearance(change: Partial<DeviceAppearance>): void {
	Object.assign(appearance, change);
	saveAppearance(appearance);
}

/** Takes on the appearance chosen in another window of this page: the
 * desktop app's tray menu stays loaded while hidden, so it would keep the
 * look it opened with. (Already saved there, so not saved again here.)
 * Returns what stops listening. */
export function followAppearance(): () => void {
	return followSavedAppearance((next) => Object.assign(appearance, next));
}
