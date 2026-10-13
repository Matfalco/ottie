<script lang="ts">
	import type { Snippet } from 'svelte';
	import icon from '#lib/assets/ottie.svg';
	import { appWindow } from '#lib/window.js';
	import WindowControls from './WindowControls.svelte';

	interface Props {
		/** What goes after the logo (the tabs and the speed). */
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
		display: flex;
		align-items: center;
		gap: 1.1rem;
		height: 3rem;
		padding-left: 0.9rem;
		background: var(--bg);
		user-select: none;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		height: 100%;
		font-size: 1.05rem;
		font-weight: 600;
		cursor: default;
	}

	.brand img {
		border-radius: 6px;
	}

	.middle {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 1rem;
		min-width: 0;
		height: 100%;
		padding-right: 0.75rem;
	}

	/* The desktop window's buttons sit right at the edge. */
	.desktop .middle {
		padding-right: 0.25rem;
	}
</style>
