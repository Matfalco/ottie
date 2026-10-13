// The appearance chosen on this device, shared by the layout (which applies
// it) and the Settings page (which changes it).

import { loadAppearance, saveAppearance, type Appearance } from './theme';

/** Change it only through `setAppearance`, so the choice is remembered. */
export const appearance: Appearance = $state(loadAppearance());

export function setAppearance(change: Partial<Appearance>): void {
	Object.assign(appearance, change);
	saveAppearance(appearance);
}
