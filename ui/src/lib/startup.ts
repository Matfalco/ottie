// What starts when one signs in to the computer (docs/decisions/0039), as
// Settings and the first-start setup offer it.

import { m } from '#lib/paraglide/messages.js';
import type { Startup } from './window.js';

/** The choices, in the order shown. */
export const STARTUP_CHOICES: Startup[] = ['tray', 'daemon', 'off'];

/** What the first-start setup has chosen until one changes it. */
export const FIRST_STARTUP: Startup = 'tray';

export const STARTUP_NAMES: Record<Startup, { name: () => string; hint: () => string }> = {
	tray: { name: m.set_startup_tray, hint: m.set_startup_tray_hint },
	daemon: { name: m.set_startup_daemon, hint: m.set_startup_daemon_hint },
	off: { name: m.set_startup_off, hint: m.set_startup_off_hint }
};
