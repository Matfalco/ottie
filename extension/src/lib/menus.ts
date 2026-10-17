// "Download with Ottie" and "Download all links with Ottie" in the browser's
// context menu (docs/decisions/0055).

/** The links a page (or the part selected) holds, as text for Ottie's list
 * for adding several: each link's address on a line, then what's selected
 * (links written as text). Runs in the page, so it uses nothing else. */
export function linksInPage(selectedOnly: boolean): string {
	const selection = document.getSelection();
	const anchors = [...document.querySelectorAll<HTMLAnchorElement>('a[href]')].filter(
		(anchor) => !selectedOnly || (selection?.containsNode(anchor, true) ?? false)
	);
	const hrefs = anchors.map((anchor) => anchor.href).filter((href) => /^(https?|ftp):/.test(href));
	const typed = selectedOnly ? (selection?.toString() ?? '') : '';
	return [...new Set(hrefs), typed].join('\n').trim();
}

/** Where Ottie's page opens signed in with its list for adding several. */
export function batchAddress(address: string, token: string, links: string): string {
	const params = new URLSearchParams({ token, links });
	return `${address}/#${params.toString()}`;
}
