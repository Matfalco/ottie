// Pages and frames the browser asked for with a form's POST: a download from
// one can't be asked for again (Ottie sends a GET), so it stays in the
// browser (docs/decisions/0055). The download follows its request within
// moments; a minute is plenty.

const KEEP_MS = 60_000;
const MOST = 200;

export class RecentPosts {
	#posted = new Map<string, number>();

	note(url: string, now: number): void {
		this.#posted.delete(url);
		this.#posted.set(url, now);
		for (const [old, at] of this.#posted) {
			if (this.#posted.size <= MOST && now - at <= KEEP_MS) {
				break;
			}
			this.#posted.delete(old);
		}
	}

	has(url: string, now: number): boolean {
		const at = this.#posted.get(url);
		return at !== undefined && now - at <= KEEP_MS;
	}
}
