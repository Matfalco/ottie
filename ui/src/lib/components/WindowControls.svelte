<script lang="ts">
	import { onMount } from 'svelte';
	import Copy from '@lucide/svelte/icons/copy';
	import Minus from '@lucide/svelte/icons/minus';
	import Square from '@lucide/svelte/icons/square';
	import X from '@lucide/svelte/icons/x';
	import { m } from '#lib/paraglide/messages.js';
	import type { AppWindow } from '#lib/window.js';

	interface Props {
		frame: AppWindow;
	}

	let { frame }: Props = $props();
	let maximized = $state(false);

	onMount(() => frame.watchMaximized((now) => (maximized = now)));

	const maximizeLabel = $derived(maximized ? m.window_restore() : m.window_maximize());
</script>

<!-- Drawn like Windows 11's: wide, flat, full height, red on close. -->
<div class="window-controls">
	<button
		onclick={() => frame.minimize()}
		aria-label={m.window_minimize()}
		title={m.window_minimize()}
	>
		<Minus size={16} strokeWidth={1.25} />
	</button>
	<button onclick={() => frame.toggleMaximize()} aria-label={maximizeLabel} title={maximizeLabel}>
		{#if maximized}
			<Copy size={13} strokeWidth={1.25} />
		{:else}
			<Square size={12} strokeWidth={1.25} />
		{/if}
	</button>
	<button
		class="close"
		onclick={() => frame.close()}
		aria-label={m.window_close()}
		title={m.window_close()}
	>
		<X size={17} strokeWidth={1.25} />
	</button>
</div>

<style>
	.window-controls {
		display: flex;
		height: 3rem;
	}

	button {
		display: grid;
		place-items: center;
		width: 2.9rem;
		height: 100%;
		padding: 0;
		border: none;
		border-radius: 0;
		background: none;
		color: var(--text);
	}

	button:hover {
		background: var(--hover);
	}

	.close:hover {
		background: var(--close-hover);
		color: var(--on-close);
	}

	button:focus-visible {
		outline-offset: -2px;
	}

	/* Rounded (docs/decisions/0044): small rounded buttons. */
	:global([data-style='rounded']) .window-controls {
		align-items: center;
		gap: 0.1rem;
		height: auto;
	}

	:global([data-style='rounded']) button {
		width: 2.2rem;
		height: 2rem;
		border-radius: var(--radius-s);
	}
</style>
