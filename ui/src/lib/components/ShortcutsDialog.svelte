<script lang="ts">
	import { onMount } from 'svelte';
	import X from '@lucide/svelte/icons/x';
	import { m } from '#lib/paraglide/messages.js';
	import { pop } from '#lib/moving.svelte.js';
	import ShortcutList from './ShortcutList.svelte';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();
	let dialog: HTMLDialogElement;

	onMount(() => dialog.showModal());
</script>

<!-- "?" anywhere in the main window (docs/decisions/0046). -->
<dialog
	bind:this={dialog}
	in:pop
	out:pop
	aria-labelledby="shortcuts-title"
	oncancel={(event) => {
		event.preventDefault();
		onClose();
	}}
>
	<header>
		<h2 id="shortcuts-title">{m.shortcuts_title()}</h2>
		<button class="close" onclick={onClose} aria-label={m.window_close()} title={m.window_close()}>
			<X size={16} aria-hidden="true" />
		</button>
	</header>
	<ShortcutList />
</dialog>

<style>
	dialog {
		width: min(34rem, calc(100vw - 2rem));
		max-height: calc(100dvh - 2rem);
		padding: 1.1rem 1.25rem 1.25rem;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
		color: var(--text);
		box-shadow: var(--shadow);
	}

	dialog::backdrop {
		background: var(--backdrop);
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 0.9rem;
	}

	h2 {
		margin: 0;
		font-size: 1.1rem;
		font-weight: 600;
	}

	.close {
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	.close:hover {
		background: var(--hover);
		color: var(--text);
	}
</style>
