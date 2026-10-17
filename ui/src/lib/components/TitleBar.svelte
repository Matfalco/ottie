<script lang="ts">
	import { tick, type Snippet } from 'svelte';
	import icon from '#lib/assets/ottie.svg';
	import { appWindow } from '#lib/window.js';
	import Switcher from './Switcher.svelte';
	import WindowControls from './WindowControls.svelte';

	interface Props {
		/** What goes after the logo (the tabs and the speed), told how far
		 * the bar is folded (see `level`). */
		children?: Snippet<[number]>;
	}

	let { children }: Props = $props();
	const frame = appWindow();

	/** How far the bar is folded to fit: 0 not at all, 1 the tabs show
	 * only their icons, 2 the time slot's chip too, 3 the logo and the
	 * switcher lose their names, 4 the speed's chip shortens to fit (long
	 * units: "kilobaiti 300 kwa kila sekunde"). What's inside folds through
	 * classes only, never
	 * by adding or removing elements (that would set off `fit` again). */
	let level = $state(0);
	const MOST = 4;
	let middle: HTMLElement | undefined = $state();
	let fitting = false;

	/** Whether what's in the middle fits in it, side by side. One that's
	 * fixed elsewhere (the tabs at the bottom, on phones) takes no room. */
	function fits(box: HTMLElement): boolean {
		const style = getComputedStyle(box);
		const room = box.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
		const items = [...box.children].filter((item) => getComputedStyle(item).position !== 'fixed');
		const gaps = (parseFloat(style.columnGap) || 0) * Math.max(0, items.length - 1);
		const used = items.reduce((sum, item) => sum + item.getBoundingClientRect().width, 0);
		return used + gaps <= room + 0.5;
	}

	/** Unfolds, then folds a step at a time until it fits; all before the
	 * next paint, so nothing flickers. */
	async function fit() {
		if (fitting || !middle) {
			return;
		}
		fitting = true;
		for (let next = 0; next <= MOST; next++) {
			level = next;
			await tick();
			if (fits(middle)) {
				break;
			}
		}
		fitting = false;
	}

	$effect(() => {
		if (!middle) {
			return;
		}
		const resized = new ResizeObserver(() => void fit());
		resized.observe(middle);
		// What's inside changes too: the speed, a slot's name, a badge.
		const changed = new MutationObserver(() => void fit());
		changed.observe(middle, { childList: true, subtree: true, characterData: true });
		return () => {
			resized.disconnect();
			changed.disconnect();
		};
	});
</script>

<!-- In the desktop app this row is the window's title bar: dragging its empty
     parts moves the window, double-clicking them maximizes it. -->
<header class:desktop={frame !== null} data-tauri-drag-region>
	<div class="brand" class:bare={level >= 3} data-tauri-drag-region="deep">
		<img src={icon} alt="" width="22" height="22" />
		<span>Ottie</span>
	</div>
	<Switcher folded={level >= 3} />
	<div class="middle" data-tauri-drag-region bind:this={middle}>
		{@render children?.(level)}
	</div>
	{#if frame}
		<WindowControls {frame} />
	{/if}
</header>

<style>
	.brand {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		height: 100%;
		font-size: 1.05rem;
		font-weight: 600;
		cursor: default;
	}

	.brand.bare span {
		display: none;
	}

	/* Whatever still doesn't fit is cut off rather than drawn over the
	   window's buttons (only sideways: the speed's popup hangs below). */
	.middle {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 1rem;
		min-width: 0;
		height: 100%;
		padding-inline-end: 0.75rem;
		overflow-x: clip;
	}

	/* The desktop window's buttons sit right at the edge. */
	.desktop .middle {
		padding-inline-end: 0.25rem;
	}

	/* On the window's colour, a little taller,
	   the icon lifted by its shadow. */
	header {
		display: flex;
		align-items: center;
		gap: 1.1rem;
		padding-inline-start: 0.9rem;
		user-select: none;
		height: 3.5rem;
		background: var(--frame);
	}

	.brand img {
		width: 1.75rem;
		height: 1.75rem;
		border-radius: 0;
		filter: drop-shadow(0 3px 8px rgb(1 107 145 / 35%));
	}

	.desktop {
		padding-inline-end: 0.4rem;
	}
</style>
