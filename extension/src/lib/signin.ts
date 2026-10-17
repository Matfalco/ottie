// Signing the extension in with a link from Ottie (docs/decisions/0055): a
// device's sign-in link (Settings → Connection) or `ottie link`'s, both
// `http://host:port/#token=…`.

import { tokenFromFragment } from '#ui/token.js';
import type { Pairing } from './settings';

/** The address and token in a sign-in link; null if it isn't one. */
export function parseSignInLink(text: string): Pairing | null {
	let url: URL;
	try {
		url = new URL(text.trim());
	} catch {
		return null;
	}
	const token = tokenFromFragment(url.hash);
	if ((url.protocol !== 'http:' && url.protocol !== 'https:') || token === null) {
		return null;
	}
	return { address: url.origin, token };
}
