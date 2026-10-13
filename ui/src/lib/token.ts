// The API token: from a link like `http://127.0.0.1:16889/#token=…` (printed
// by `ottie serve`, opened by `ottie open`), or typed on the login page.
// Remembered in this browser until the daemon refuses it.

const KEY = 'ottie.token';

/** The token in a `#token=…` fragment, if there is one. (The part after `#`
 * never leaves the browser, so it doesn't reach any server log.) */
export function tokenFromFragment(hash: string): string | null {
	const match = /^#token=([\w.~-]+)$/.exec(hash);
	return match ? match[1] : null;
}

/** The address that opens this page signed in, as `ottie link` prints it:
 * the page's own address, with the token after `#`. */
export function signInLink(origin: string, token: string): string {
	return `${origin}/#token=${token}`;
}

export function loadToken(): string | null {
	try {
		return localStorage.getItem(KEY);
	} catch {
		return null; // Storage blocked: sign in again next time.
	}
}

export function saveToken(token: string): void {
	try {
		localStorage.setItem(KEY, token);
	} catch {
		// Storage blocked: it lasts until the page is closed.
	}
}

export function forgetToken(): void {
	try {
		localStorage.removeItem(KEY);
	} catch {
		// Nothing was stored.
	}
}
