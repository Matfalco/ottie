<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { pop } from '#lib/moving.svelte.js';

	interface Props {
		title: string;
		/** The name to start from (renaming), or ''. */
		name: string;
		/** A line under the field, if any. */
		hint?: string;
		/** Resolves to null once saved, else to why not (shown here). */
		onSave: (name: string) => Promise<string | null>;
		onClose: () => void;
	}

	let { title, name, hint, onSave, onClose }: Props = $props();
	let dialog: HTMLDialogElement;
	let typed = $state(untrack(() => name));
	let problem: string | null = $state(null);
	let saving = $state(false);

	onMount(() => dialog.showModal());

	async function save(event: SubmitEvent) {
		event.preventDefault();
		saving = true;
		problem = await onSave(typed);
		saving = false;
		if (problem === null) {
			onClose();
		}
	}
</script>

<dialog
	bind:this={dialog}
	in:pop
	out:pop
	aria-labelledby="view-dialog-title"
	oncancel={(event) => {
		// Escape: the page decides when it closes.
		event.preventDefault();
		onClose();
	}}
>
	<form onsubmit={save}>
		<h2 id="view-dialog-title">{title}</h2>
		<label>
			<span>{m.view_name()}</span>
			<!-- svelte-ignore a11y_autofocus -->
			<input
				bind:value={typed}
				dir="auto"
				placeholder={m.view_name_example()}
				aria-invalid={problem !== null}
				autofocus
			/>
		</label>
		{#if problem}
			<p class="problem" role="alert">{problem}</p>
		{:else if hint}
			<p class="hint">{hint}</p>
		{/if}
		<div class="buttons">
			<button type="button" onclick={onClose}>{m.view_cancel()}</button>
			<button class="primary" disabled={saving || typed.trim() === ''}>{m.view_save()}</button>
		</div>
	</form>
</dialog>

<style>
	dialog {
		width: min(24rem, calc(100vw - 2rem));
		padding: 1.25rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
		color: var(--text);
		box-shadow: var(--shadow);
	}

	dialog::backdrop {
		background: var(--backdrop);
	}

	form {
		display: grid;
		gap: 0.75rem;
	}

	h2 {
		margin: 0;
		font-size: 1.1em;
		font-weight: 600;
	}

	label {
		display: grid;
		gap: 0.3rem;
		font-size: 0.9em;
		font-weight: 500;
	}

	input {
		font-size: 1rem;
	}

	p {
		margin: 0;
		font-size: 0.85em;
	}

	.hint {
		color: var(--muted);
	}

	.problem {
		color: var(--danger);
	}

	.buttons {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 0.25rem;
	}
</style>
