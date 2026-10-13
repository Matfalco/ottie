<script lang="ts">
	import type { Snippet } from 'svelte';
	import icon from '#lib/assets/ottie.svg';
	import { appWindow } from '#lib/window.js';
	import WindowControls from './WindowControls.svelte';

	interface Props {
		/** What goes in the middle (the add bar). */
		children?: Snippet;
	}

	let { children }: Props = $props();
	const frame = appWindow();
</script>

<!-- In the desktop app this row is the window's title bar: dragging its empty
     parts moves the window, double-clicking them maximizes it. -->
<header class:desktop={frame !== null} data-tauri-drag-region>
	<div class="brand" data-tauri-drag-region="deep">
		<img src={icon} alt="" width="22" height="22" />
		<span>Ottie</span>
	</div>
	<div class="middle" data-tauri-drag-region>
		{@render children?.()}
	</div>
	{#if frame}
		<WindowControls {frame} />
	{/if}
</header>

<style>
	header {
		display: grid;
		grid-template-columns: 13rem minmax(0, 1fr) auto;
		align-items: center;
		height: 3rem;
		background: var(--chrome);
		border-bottom: 1px solid var(--border);
		user-select: none;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		height: 100%;
		padding: 0 1rem;
		font-weight: 600;
		cursor: default;
	}

	.brand img {
		border-radius: 6px;
	}

	.middle {
		display: flex;
		align-items: center;
		height: 100%;
		padding: 0 0.75rem 0 0;
	}

	/* Narrow: the add bar gets a row of its own under the logo. */
	@media (max-width: 40rem) {
		header {
			grid-template-columns: 1fr auto;
			grid-template-areas: 'brand controls' 'middle middle';
			height: auto;
		}

		.brand {
			grid-area: brand;
			height: 2.75rem;
		}

		.middle {
			grid-area: middle;
			padding: 0 0.75rem 0.6rem;
		}

		header :global(.window-controls) {
			grid-area: controls;
			align-self: start;
		}
	}
</style>
