// The `Cookie` header a browser would send with a link, from its cookies
// for that link (docs/decisions/0055).

export interface CookieLike {
	name: string;
	value: string;
	path: string;
}

/** As browsers order them: longer paths first, else as given. null: none. */
export function cookieHeader(cookies: readonly CookieLike[]): string | null {
	if (cookies.length === 0) {
		return null;
	}
	return [...cookies]
		.sort((a, b) => b.path.length - a.path.length)
		.map(({ name, value }) => (name === '' ? value : `${name}=${value}`))
		.join('; ');
}
