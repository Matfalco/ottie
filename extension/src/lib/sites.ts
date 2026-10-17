// Sites whose downloads stay in the browser (docs/decisions/0055).

/** A site as typed ("example.com", "https://www.example.com/files") as the
 * host name kept; null if there's none in it. */
export function siteOf(text: string): string | null {
	const trimmed = text.trim().toLowerCase();
	if (trimmed === '') {
		return null;
	}
	try {
		const url = new URL(trimmed.includes('://') ? trimmed : `http://${trimmed}`);
		return url.hostname === '' ? null : url.hostname.replace(/^www\./, '');
	} catch {
		return null;
	}
}

/** The sites in a text, one per line (or separated by commas or spaces),
 * each once, in order. */
export function parseSites(text: string): string[] {
	const sites = text.split(/[\s,]+/).map(siteOf);
	return [...new Set(sites.filter((site): site is string => site !== null))];
}

/** Whether `url`'s host is one of `sites` or under one. */
export function isExcluded(url: string, sites: readonly string[]): boolean {
	let host: string;
	try {
		host = new URL(url).hostname.toLowerCase();
	} catch {
		return false;
	}
	return sites.some((site) => host === site || host.endsWith(`.${site}`));
}
