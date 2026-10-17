// The popup shows Ottie's own `/popup` page in a frame (docs/decisions/0055),
// which asks for the token once it's ready and says how tall it is.

/** What the page says, from `ui/src/lib/popup.ts`; null for anything else. */
export type FromPage =
	| { type: 'ottie-popup-ready' }
	| { type: 'ottie-popup-size'; height: number }
	| { type: 'ottie-popup-refused' };

export function fromPage(data: unknown): FromPage | null {
	if (typeof data !== 'object' || data === null) {
		return null;
	}
	const { type, height } = data as { type?: unknown; height?: unknown };
	switch (type) {
		case 'ottie-popup-ready':
		case 'ottie-popup-refused':
			return { type };
		case 'ottie-popup-size':
			return typeof height === 'number' && Number.isFinite(height) ? { type, height } : null;
		default:
			return null;
	}
}

/** Browsers cap a popup at 600 pixels high; the bar above takes the rest. */
export const MOST_HEIGHT = 540;

export function frameHeight(height: number): number {
	return Math.min(MOST_HEIGHT, Math.max(48, Math.ceil(height)));
}
