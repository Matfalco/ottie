// The desktop app around the page, when it runs inside it. The app hides the
// system title bar (docs/decisions/0019), so the page draws its own and needs
// the window's buttons; it can also open the system folder picker. In a
// browser there's neither. It also chooses what plays a download, and hears
// from the tray.

import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import type { Sound, SoundKind, Sounds } from './sounds.js';

export interface AppWindow {
	minimize(): Promise<void>;
	toggleMaximize(): Promise<void>;
	close(): Promise<void>;
	/** Calls `onChange` with whether the window is maximized, now and after
	 * every resize. Returns a function that stops watching. */
	watchMaximized(onChange: (maximized: boolean) => void): () => void;
}

/** The window around this page, or null in a browser. */
export function appWindow(): AppWindow | null {
	if (!isTauri()) {
		return null;
	}
	const window = getCurrentWindow();
	return {
		minimize: () => window.minimize(),
		toggleMaximize: () => window.toggleMaximize(),
		close: () => window.close(),
		watchMaximized(onChange) {
			let stopped = false;
			let unlisten: (() => void) | null = null;
			const check = () => {
				// A failed check leaves the button as it was.
				window.isMaximized().then(
					(maximized) => !stopped && onChange(maximized),
					() => {}
				);
			};
			check();
			window.onResized(check).then(
				(stop) => (stopped ? stop() : (unlisten = stop)),
				() => {}
			);
			return () => {
				stopped = true;
				unlisten?.();
			};
		}
	};
}

/** Whether the system folder picker is there (only in the desktop app). */
export function canPickFolder(): boolean {
	return isTauri();
}

/** Asks for a folder with the system picker; null if cancelled. Only valid
 * for a daemon on this computer, which the desktop app always uses so far. */
export async function pickFolder(start?: string): Promise<string | null> {
	const chosen = await open({ directory: true, defaultPath: start || undefined });
	return typeof chosen === 'string' ? chosen : null;
}

/** What "Play" opens in the desktop app (docs/decisions/0031). */
export type PlayWith = 'system' | 'browser' | 'program';

export interface Player {
	playWith: PlayWith;
	/** The program picked last, if any. */
	program: string | null;
}

/** Whether "Play" can open a player here (only in the desktop app). */
export function canChoosePlayer(): boolean {
	return isTauri();
}

/** How the desktop app plays links. */
export function player(): Promise<Player> {
	return invoke<Player>('player');
}

/** Plays with the system's player, the browser, or the program already picked. */
export function setPlayWith(playWith: PlayWith): Promise<Player> {
	return invoke<Player>('set_play_with', { playWith });
}

/** Asks for a program with the system's picker; unchanged if cancelled.
 * `programs` names the file filter (on Windows). */
export function choosePlayer(title: string, programs: string): Promise<Player> {
	return invoke<Player>('choose_player', { title, programs });
}

/** When the desktop app notifies finished downloads (docs/decisions/0035):
 * as each one finishes, once nothing runs or waits any more, or never. */
export type Notify = 'each' | 'all' | 'never';

/** What happens when one signs in to the computer (docs/decisions/0039):
 * the app starts in the tray, only the daemon starts, or nothing. */
export type Startup = 'tray' | 'daemon' | 'off';

/** The desktop app's own settings (docs/decisions/0034, 0035, 0038, 0039). */
export interface Desktop {
	startup: Startup;
	/** Closing the window keeps Ottie running in the tray. */
	closeToTray: boolean;
	notify: Notify;
	/** A failed download is notified (after its last retry). */
	notifyFailed: boolean;
	sounds: Sounds;
	/** With `custom` sounds: one for each kind. */
	soundFinished: Sound;
	soundFailed: Sound;
	/** Ottie's sound sets, by ID, to choose from. */
	sets: string[];
	/** The system's sound can be played to hear it (on Windows). */
	systemSound: boolean;
}

export function desktop(): Promise<Desktop> {
	return invoke<Desktop>('desktop');
}

/** Whether Ottie can start at sign-in (only the desktop app). */
export function canStartAtSignIn(): boolean {
	return isTauri();
}

export function setStartup(startup: Startup): Promise<Desktop> {
	return invoke<Desktop>('set_startup', { startup });
}

export function setCloseToTray(on: boolean): Promise<Desktop> {
	return invoke<Desktop>('set_close_to_tray', { on });
}

export function setNotify(notify: Notify): Promise<Desktop> {
	return invoke<Desktop>('set_notify', { notify });
}

export function setNotifyFailed(on: boolean): Promise<Desktop> {
	return invoke<Desktop>('set_notify_failed', { on });
}

export function setSounds(sounds: Sounds): Promise<Desktop> {
	return invoke<Desktop>('set_sounds', { sounds });
}

/** With `custom` sounds: the sound of one kind. */
export function setSound(kind: SoundKind, sound: Sound): Promise<Desktop> {
	return invoke<Desktop>('set_sound', { kind, sound });
}

/** Plays a sound to hear it (see `canPreview`). A failure stays quiet. */
export function playSound(sound: Sound): void {
	invoke('play_sound', { sound }).catch(() => {});
}

/** Tells the desktop app which language the page shows, for the tray's
 * texts. Nothing in a browser; a failure leaves the tray as it was. */
export function tellLanguage(language: string): void {
	if (isTauri()) {
		invoke('set_language', { language }).catch(() => {});
	}
}

/** Ottie's own tray menu (docs/decisions/0034), drawn by the `/tray` page:
 * says how tall it is, so the app can size and place its window. */
export function menuReady(height: number): void {
	invoke('menu_ready', { height }).catch(() => {});
}

/** The menu's "Open Ottie": the main window, brought forward. */
export function showMain(): Promise<void> {
	return invoke('show_main');
}

/** The menu's "Quit Ottie". */
export function quitApp(): Promise<void> {
	return invoke('quit_app');
}

/** Closes the menu, as a click elsewhere does. */
export function hideMenu(): Promise<void> {
	return getCurrentWindow().hide();
}
