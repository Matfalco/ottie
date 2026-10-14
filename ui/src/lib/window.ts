// The desktop app around the page, when it runs inside it. The app hides the
// system title bar (docs/decisions/0019), so the page draws its own and needs
// the window's buttons; it can also open the system folder picker. In a
// browser there's neither. It also chooses what plays a download, and hears
// from the tray.

import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';

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

/** The desktop app's own settings (docs/decisions/0034). */
export interface Desktop {
	/** Closing the window keeps Ottie running in the tray. */
	closeToTray: boolean;
}

export function desktop(): Promise<Desktop> {
	return invoke<Desktop>('desktop');
}

export function setCloseToTray(on: boolean): Promise<Desktop> {
	return invoke<Desktop>('set_close_to_tray', { on });
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
