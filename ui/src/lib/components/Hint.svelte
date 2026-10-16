<script lang="ts">
	import Info from '@lucide/svelte/icons/info';
	import { bubbleAt, type Box } from '#lib/hint.js';

	interface Props {
		/** What the setting does: the button's name, and the bubble. */
		text: string;
	}

	let { text }: Props = $props();
	let button: HTMLButtonElement;
	let bubble: HTMLElement | undefined = $state();
	/** The mouse is over the button. */
	let hovering = $state(false);
	/** Clicked (or tapped) open: stays until clicked again or left. */
	let pinned = $state(false);
	let open = $derived(hovering || pinned);
	let at: Box | null = $state(null);

	function close() {
		hovering = false;
		pinned = false;
	}

	// Placed once drawn (its size is known then), inside the window.
	$effect(() => {
		if (!open || !bubble) {
			at = null;
			return;
		}
		at = bubbleAt(
			button.getBoundingClientRect(),
			bubble.getBoundingClientRect(),
			{ width: innerWidth, height: innerHeight },
			getComputedStyle(button).direction === 'rtl'
		);
	});

	// It doesn't follow a scrolling page: it closes instead, as it does on a
	// click elsewhere.
	$effect(() => {
		if (!open) {
			return;
		}
		const elsewhere = (event: PointerEvent) => {
			if (event.target instanceof Node && !button.contains(event.target)) {
				close();
			}
		};
		addEventListener('scroll', close, true);
		addEventListener('resize', close);
		addEventListener('pointerdown', elsewhere, true);
		return () => {
			removeEventListener('scroll', close, true);
			removeEventListener('resize', close);
			removeEventListener('pointerdown', elsewhere, true);
		};
	});
</script>

<!-- The (i) beside a setting's name (docs/decisions/0044): its explanation
     on hover, or on a click or tap. -->
<button
	type="button"
	class="info"
	aria-label={text}
	bind:this={button}
	onpointerenter={(event) => (hovering = event.pointerType === 'mouse')}
	onpointerleave={() => (hovering = false)}
	onclick={() => {
		pinned = !pinned;
		if (!pinned) {
			hovering = false;
		}
	}}
	onkeydown={(event) => event.key === 'Escape' && close()}
	onblur={close}
>
	<Info size={13} aria-hidden="true" />
</button>
{#if open}
	<!-- The button's name says it already. -->
	<span
		class="bubble"
		aria-hidden="true"
		bind:this={bubble}
		style:left={at ? `${at.left}px` : '0'}
		style:top={at ? `${at.top}px` : '0'}
		style:visibility={at ? 'visible' : 'hidden'}>{text}</span
	>
{/if}

<style>
	.info {
		display: grid;
		flex: none;
		place-items: center;
		width: 1.25rem;
		height: 1.25rem;
		padding: 0;
		border: none;
		border-radius: 50%;
		color: var(--faint);
	}

	.info:hover,
	.info:focus-visible {
		background: var(--hover);
		color: var(--text);
	}

	/* Fixed to the window, where the script puts it. */
	.bubble {
		position: fixed;
		z-index: 20;
		width: max-content;
		max-width: min(18rem, calc(100vw - 16px));
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-m);
		background: var(--surface);
		box-shadow: var(--shadow);
		color: var(--text);
		font-size: 0.8rem;
		font-weight: 400;
		line-height: 1.45;
		text-align: start;
		white-space: normal;
		pointer-events: none;
	}

	.info {
		background: var(--hover);
	}
</style>
