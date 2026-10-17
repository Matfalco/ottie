// The desktop app around the page, when it runs inside it. The app hides the
// system title bar (docs/decisions/0019), so the page draws its own and needs
// the window's buttons; it can also open the system folder picker. In a
// browser there's neither. It also opens a download's file or folder,
// chooses what plays a download, hears from the tray, and offers links
// copied elsewhere. The window can show another computer's Ottie
// (docs/decisions/0053): its pages get only the window, the tray menu, the
// clipboard's window and switching, never what reaches this computer.

import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import { asRefusal, type Daemons, type OthersRefusal } from './others.js';
import type { Sound, SoundKind, Sounds } from './sounds.js';

declare global {
	interface Window {
		/** This computer's Ottie's address, set by the desktop app in every
		 * page it opens. */
		__OTTIE_LOCAL__?: string;
	}
}

/** In the desktop app, on this computer's Ottie's page: its files, its
 * programs and its power are within reach. Not on another computer's page,
 * nor in a browser. */
export function onThisComputer(): boolean {
	return isTauri() && window.__OTTIE_LOCAL__ === location.origin;
}

/** In the desktop app, on any Ottie's page. */
export function inApp(): boolean {
	return isTauri();
}

/** This computer's Ottie's address, in the desktop app. */
export function localAddress(): string | null {
	return isTauri() ? (window.__OTTIE_LOCAL__ ?? null) : null;
}

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

/** Whether the system folder picker is there (only in the desktop app, on
 * this computer's Ottie: another one's folders are on that computer). */
export function canPickFolder(): boolean {
	return onThisComputer();
}

/** Asks for a folder with the system picker; null if cancelled. */
export async function pickFolder(start?: string): Promise<string | null> {
	const chosen = await open({ directory: true, defaultPath: start || undefined });
	return typeof chosen === 'string' ? chosen : null;
}

/** Why a download's file or folder couldn't open (docs/decisions/0045):
 * it's not there any more, or the app couldn't. */
export type OpenRefusal = 'gone' | 'failed';

/** Whether downloads' files and folders can be opened here (only in the
 * desktop app, on this computer's Ottie). */
export function canOpenFiles(): boolean {
	return onThisComputer();
}

/** Opens a finished download's file; the app asks first before running a
 * program. Resolves to null, or why it couldn't. */
export function openFile(id: string): Promise<OpenRefusal | null> {
	return refusal(invoke('open_file', { id }));
}

/** Opens the download's folder with its file selected. */
export function showInFolder(id: string): Promise<OpenRefusal | null> {
	return refusal(invoke('show_in_folder', { id }));
}

async function refusal(done: Promise<unknown>): Promise<OpenRefusal | null> {
	try {
		await done;
		return null;
	} catch (why) {
		return why === 'gone' ? 'gone' : 'failed';
	}
}

/** What "Play" opens in the desktop app (docs/decisions/0031). */
export type PlayWith = 'system' | 'browser' | 'program';

export interface Player {
	playWith: PlayWith;
	/** The program picked last, if any. */
	program: string | null;
}

/** Whether the desktop app's own settings are within reach: the player,
 * the tray, notifications, sign-in (only on this computer's Ottie). */
export function canChoosePlayer(): boolean {
	return onThisComputer();
}

/** Whether "Play" goes to the player chosen in the desktop app (any Ottie
 * shown in it: a play link needs no token). */
export function playsInApp(): boolean {
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

/** What a link copied elsewhere does (docs/decisions/0043): it's offered
 * in a small window, or added at once. */
export type Copied = 'ask' | 'add';

/** Whether this computer's Ottie starts with the app while the window
 * shows another computer's (docs/decisions/0053). */
export type LocalDaemon = 'always' | 'whenShown';

/** The desktop app's own settings (docs/decisions/0034, 0035, 0038, 0039,
 * 0043). */
export interface Desktop {
	startup: Startup;
	/** Closing the window keeps Ottie running in the tray. */
	closeToTray: boolean;
	/** Links copied elsewhere are caught (to file types a category lists). */
	watchClipboard: boolean;
	copied: Copied;
	localDaemon: LocalDaemon;
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

/** Calls `onChange` whenever they change, in any window. Returns a function
 * that stops listening. */
export function followDesktop(onChange: (desktop: Desktop) => void): () => void {
	return follow<Desktop>('desktop', onChange);
}

/** Whether Ottie can start at sign-in (only the desktop app, on this
 * computer's Ottie). */
export function canStartAtSignIn(): boolean {
	return onThisComputer();
}

export function setStartup(startup: Startup): Promise<Desktop> {
	return invoke<Desktop>('set_startup', { startup });
}

export function setCloseToTray(on: boolean): Promise<Desktop> {
	return invoke<Desktop>('set_close_to_tray', { on });
}

export function setWatchClipboard(on: boolean): Promise<Desktop> {
	return invoke<Desktop>('set_watch_clipboard', { on });
}

export function setCopied(copied: Copied): Promise<Desktop> {
	return invoke<Desktop>('set_copied', { copied });
}

export function setLocalDaemon(local: LocalDaemon): Promise<Desktop> {
	return invoke<Desktop>('set_local_daemon', { local });
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

/** What happens when everything has finished (docs/decisions/0040). */
export type Finish = 'nothing' | 'quit' | 'sleep' | 'hibernate' | 'shutDown';
/** Those that need the computer, after a countdown. */
export type Power = Exclude<Finish, 'nothing' | 'quit'>;
export type CountdownIn = 'window' | 'notification';

/** The desktop app's "when everything has finished": what's chosen, what
 * this computer can do, and the countdown while one runs. */
export interface Finishing {
	/** Every time (Settings). */
	always: Finish;
	/** This time only, instead (the tray menu, the Downloads page). */
	once: Finish | null;
	/** What this computer lets Ottie do now. */
	able: Record<Power, boolean>;
	/** The countdown's length, in seconds. */
	countdown: number;
	countdownIn: CountdownIn;
	/** Downloads waiting for their time or held by a slot count as not finished. */
	waitsForScheduled: boolean;
	/** While a countdown runs: what, and when (milliseconds since 1970). */
	running: { action: Power; endsAt: number } | null;
}

/** Whether something can happen when everything has finished (only the
 * desktop app, on this computer's Ottie: it's this computer that sleeps). */
export function canFinish(): boolean {
	return onThisComputer();
}

export function finishing(): Promise<Finishing> {
	return invoke<Finishing>('finish');
}

/** Calls `onChange` whenever any of it changes, in any window. Returns a
 * function that stops listening. */
export function followFinishing(onChange: (finishing: Finishing) => void): () => void {
	return follow<Finishing>('finish', onChange);
}

/** Calls `onEvent` with each of the app's `name` events. Returns a function
 * that stops listening. */
function follow<T>(name: string, onEvent: (payload: T) => void): () => void {
	let stopped = false;
	let unlisten: (() => void) | null = null;
	listen<T>(name, (event) => onEvent(event.payload)).then(
		(stop) => (stopped ? stop() : (unlisten = stop)),
		() => {}
	);
	return () => {
		stopped = true;
		unlisten?.();
	};
}

export function setFinish(finish: Finish): Promise<Finishing> {
	return invoke<Finishing>('set_finish', { finish });
}

export function setFinishOnce(finish: Finish): Promise<Finishing> {
	return invoke<Finishing>('set_finish_once', { finish });
}

export function setCountdown(seconds: number): Promise<Finishing> {
	return invoke<Finishing>('set_countdown', { seconds });
}

export function setCountdownIn(countdownIn: CountdownIn): Promise<Finishing> {
	return invoke<Finishing>('set_countdown_in', { countdownIn });
}

export function setFinishWaits(on: boolean): Promise<Finishing> {
	return invoke<Finishing>('set_finish_waits', { on });
}

/** Cancels the countdown: the computer stays as it is. */
export function cancelFinish(): Promise<void> {
	return invoke('cancel_finish');
}

/** Ends the countdown now: the computer goes. */
export function finishNow(): Promise<void> {
	return invoke('finish_now');
}

/** The countdown's window: says how tall it is, so the app can size it. */
export function countdownReady(height: number): void {
	invoke('countdown_ready', { height }).catch(() => {});
}

/** The links copied elsewhere that the clipboard's window offers
 * (docs/decisions/0043). */
export function caughtLinks(): Promise<string[]> {
	return invoke<string[]>('caught');
}

/** Calls `onChange` when they change (ask `caughtLinks` again). Returns a
 * function that stops listening. */
export function followCaught(onChange: () => void): () => void {
	return follow<unknown>('caught', onChange);
}

/** The clipboard's window: says how tall it is, so the app can size and
 * place it (and show it). */
export function caughtReady(height: number): void {
	invoke('caught_ready', { height }).catch(() => {});
}

/** These links are dealt with (added or ignored): the window hides unless
 * others came meanwhile. */
export function caughtDone(links: string[]): Promise<void> {
	return invoke('caught_done', { links });
}

/** These links go to the main window's list, to choose their categories
 * and folder. */
export function caughtMore(links: string[]): Promise<void> {
	return invoke('caught_more', { links });
}

/** In the main window: calls `onLinks` with the links' text when the
 * clipboard's window sends them (More…). Nothing in a browser. Returns a
 * function that stops listening. */
export function followAddLinks(onLinks: (text: string) => void): () => void {
	return isTauri() ? follow<string>('add-links', onLinks) : () => {};
}

/** The desktop app's list of other computers' Ottie, and which one its
 * window shows (docs/decisions/0053). */
export function daemons(): Promise<Daemons> {
	return invoke<Daemons>('daemons');
}

/** Calls `onChange` whenever the list or the one shown changes. Returns a
 * function that stops listening. */
export function followDaemons(onChange: (daemons: Daemons) => void): () => void {
	return follow<Daemons>('daemons', onChange);
}

/** Shows another computer's Ottie in the window, or this computer's (null):
 * this page goes. Resolves to why it couldn't, or null. */
export async function switchDaemon(id: number | null): Promise<OthersRefusal | null> {
	try {
		await invoke('switch_daemon', { id });
		return null;
	} catch (why) {
		return asRefusal(why);
	}
}

/** Adds one from a sign-in link, once it answers and takes the token. */
export async function addDaemon(name: string, link: string): Promise<Daemons | OthersRefusal> {
	try {
		return await invoke<Daemons>('add_daemon', { name, link });
	} catch (why) {
		return asRefusal(why);
	}
}

/** Takes one off the list (the window comes back to this computer's first
 * if it showed it). */
export async function removeDaemon(id: number): Promise<Daemons | OthersRefusal> {
	try {
		return await invoke<Daemons>('remove_daemon', { id });
	} catch (why) {
		return asRefusal(why);
	}
}
