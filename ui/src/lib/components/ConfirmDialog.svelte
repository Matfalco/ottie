<script lang="ts">
	import { onMount } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		message: string;
		/** The button that goes ahead, e.g. "Remove and delete". */
		confirmLabel: string;
		onAnswer: (yes: boolean) => void;
	}

	let { message, confirmLabel, onAnswer }: Props = $props();
	let dialog: HTMLDialogElement;

	// Modal: the rest of the page can't be used until it's answered. Focus
	// starts on "Go back", the safe choice.
	onMount(() => dialog.showModal());
</script>

<dialog
	bind:this={dialog}
	oncancel={(event) => {
		// Escape: answer "no" ourselves, so the page decides when it closes.
		event.preventDefault();
		onAnswer(false);
	}}
>
	<p>{message}</p>
	<div class="buttons">
		<button onclick={() => onAnswer(false)}>{m.confirm_back()}</button>
		<button class="danger" onclick={() => onAnswer(true)}>{confirmLabel}</button>
	</div>
</dialog>

<style>
	dialog {
		width: min(26rem, calc(100vw - 2rem));
		padding: 1.25rem;
		border: 1px solid var(--border);
		border-radius: 12px;
		background: var(--surface);
		color: var(--text);
		box-shadow: var(--shadow);
	}

	dialog::backdrop {
		background: var(--backdrop);
	}

	p {
		margin: 0 0 1.25rem;
	}

	.buttons {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	.danger {
		border-color: transparent;
		background: var(--danger-strong);
		color: var(--on-close);
	}
</style>
