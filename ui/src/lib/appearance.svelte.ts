// The appearance chosen on this device, shared by the layout (which applies
// it), the shell (which follows the shared one) and the Settings page (which
// changes it).

import { loadAppearance, saveAppearance, type DeviceAppearance } from './theme';

/** Change it only through `setAppearance`, so the choice is remembered. */
export const appearance: DeviceAppearance = $state(loadAppearance());

export function setAppearance(change: Partial<DeviceAppearance>): void {
	Object.assign(appearance, change);
	saveAppearance(appearance);
}
