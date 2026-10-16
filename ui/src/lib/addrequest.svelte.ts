// What a shortcut asks of the add bar (docs/decisions/0046): a pasted link
// to add, or the cursor in its field. The shell asks (going to Downloads
// first from a page without an add bar); the add bar on the page takes it.

export interface Asked {
	/** The link for the field, its step opened; null: only the cursor. */
	link: string | null;
}

export class AddRequest {
	/** Asked and not yet taken. */
	pending: Asked | null = $state(null);

	ask(link: string | null) {
		this.pending = { link };
	}

	/** The request, once. */
	take(): Asked | null {
		const asked = this.pending;
		this.pending = null;
		return asked;
	}
}
