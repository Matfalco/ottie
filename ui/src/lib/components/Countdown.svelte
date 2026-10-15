<script lang="ts">
	import type { Component } from 'svelte';
	import Moon from '@lucide/svelte/icons/moon';
	import Power from '@lucide/svelte/icons/power';
	import Snowflake from '@lucide/svelte/icons/snowflake';
	import { onMount } from 'svelte';
	import { nowText, soonText } from '#lib/finish.js';
	import { LiveFinishing } from '#lib/finish.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cancelFinish, countdownReady, finishNow, type Power as Action } from '#lib/window.js';

	// The desktop app's countdown before the computer sleeps, hibernates or
	// shuts down (docs/decisions/0040): a small window on top of everything,
	// sized to what's drawn here. Cancel has the focus; the app closes the
	// window when the countdown ends or is cancelled.

	const ICONS: Record<Action, Component> = {
		sleep: Moon,
		hibernate: Snowflake,
		shutDown: Power
	};

	const live = new LiveFinishing();
	const running = $derived(live.current?.running ?? null);
	const left = $derived(live.left ?? 0);
	/** How much of the countdown is left, 0 to 1, for the bar. */
	const share = $derived(
		live.current && running ? Math.min(1, left / Math.max(1, live.current.countdown)) : 0
	);
	const Icon = $derived(running ? ICONS[running.action] : null);

	let box: HTMLElement;
	let cancelButton: HTMLButtonElement | undefined = $state();

	onMount(() => {
		live.start();
		const observer = new ResizeObserver(() =>
			countdownReady(Math.ceil(box.getBoundingClientRect().height))
		);
		observer.observe(box);
		return () => {
			observer.disconnect();
			live.stop();
		};
	});

	// The safe choice is the one a key press takes.
	$effect(() => cancelButton?.focus());

	function cancel() {
		void cancelFinish().catch(() => {});
	}

	function now() {
		void finishNow().catch(() => {});
	}

	function keyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			cancel();
		}
	}
</script>

<svelte:window onkeydown={keyDown} />

<div
	class="countdown"
	role="alertdialog"
	aria-labelledby="countdown-title"
	aria-describedby="countdown-why"
	bind:this={box}
>
	{#if running && Icon}
		<div class="head">
			<span class="icon" aria-hidden="true"><Icon size={22} /></span>
			<div class="names">
				<h1 id="countdown-title">{soonText(running.action, left)}</h1>
				<p id="countdown-why">{m.finish_done()}</p>
			</div>
		</div>
		<div class="bar" aria-hidden="true">
			<span style:inline-size="{share * 100}%"></span>
		</div>
		<div class="buttons">
			<button onclick={now}>{nowText(running.action)}</button>
			<button class="primary" bind:this={cancelButton} onclick={cancel}>{m.finish_cancel()}</button>
		</div>
	{:else}
		<p id="countdown-title" class="waiting">{m.finish_title()}</p>
		<p id="countdown-why" hidden></p>
	{/if}
</div>

<style>
	:global(html),
	:global(body) {
		background: var(--surface);
	}

	.countdown {
		display: grid;
		gap: 1rem;
		padding: 1.25rem;
		border: 1px solid var(--border);
		background: var(--surface);
		color: var(--text);
		user-select: none;
	}

	.head {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
	}

	.icon {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: 50%;
		background: var(--selected);
		color: var(--accent);
	}

	.names {
		display: grid;
		gap: 0.25rem;
	}

	h1 {
		margin: 0;
		font-size: 1.1rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}

	p {
		margin: 0;
		color: var(--muted);
		font-size: 0.9rem;
	}

	.bar {
		height: 4px;
		overflow: hidden;
		border-radius: 2px;
		background: var(--border-soft);
	}

	.bar span {
		display: block;
		height: 100%;
		background: var(--accent);
		transition: inline-size 0.25s linear;
	}

	.buttons {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	.waiting {
		padding: 0.5rem 0;
	}
</style>
