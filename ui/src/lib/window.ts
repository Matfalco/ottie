// The desktop app's window, when the page runs inside it. The app hides the
// system title bar (docs/decisions/0019), so the page draws its own and needs
// the window's buttons. In a browser there's no window to control.

import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

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
