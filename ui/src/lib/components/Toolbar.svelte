<script lang="ts">
	import type { Component } from 'svelte';
	import type { Job } from '@ottie/api-types/Job';
	import Ban from '@lucide/svelte/icons/ban';
	import Eraser from '@lucide/svelte/icons/eraser';
	import Pause from '@lucide/svelte/icons/pause';
	import Play from '@lucide/svelte/icons/play';
	import Trash from '@lucide/svelte/icons/trash';
	import { targets, type Action } from '#lib/bulk.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** The selected downloads. */
		selected: Job[];
		/** Finished downloads in the view, for "Clear finished". */
		finished: Job[];
		onRun: (action: Action, jobs: Job[]) => void;
	}

	let { selected, finished, onRun }: Props = $props();

	const BUTTONS: { action: Action; icon: Component; label: () => string }[] = [
		{ action: 'resume', icon: Play, label: m.resume },
		{ action: 'pause', icon: Pause, label: m.pause },
		{ action: 'cancel', icon: Ban, label: m.cancel },
		{ action: 'remove', icon: Trash, label: m.remove }
	];
</script>

<div class="toolbar" role="toolbar" aria-label={m.toolbar_label()}>
	{#each BUTTONS as { action, icon: Icon, label } (action)}
		<button
			disabled={targets(selected, action).length === 0}
			onclick={() => onRun(action, selected)}
			aria-label={label()}
			title={label()}
		>
			<Icon size={15} aria-hidden="true" />
			<span>{label()}</span>
		</button>
	{/each}
	{#if selected.length > 0}
		<span class="count">{m.selected_count({ count: formatCount(selected.length) })}</span>
	{/if}
	{#if finished.length > 0}
		<button class="clear" onclick={() => onRun('remove', finished)}>
			<Eraser size={15} aria-hidden="true" />
			<span>{m.clear_finished()}</span>
		</button>
	{/if}
</div>

<style>
	.toolbar {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		min-height: 2.75rem;
		padding: 0.35rem 0.75rem;
		border-bottom: 1px solid var(--border);
	}

	button {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		height: 2rem;
		padding: 0 0.6rem;
		border-color: transparent;
		background: none;
		white-space: nowrap;
	}

	button:hover:not(:disabled) {
		border-color: transparent;
		background: var(--hover);
	}

	button:disabled {
		opacity: 0.4;
	}

	.count {
		margin-left: 0.5rem;
		color: var(--muted);
		font-size: 0.9em;
		white-space: nowrap;
	}

	.clear {
		margin-left: auto;
		color: var(--muted);
	}

	/* Narrow: icons only (each button keeps its name for screen readers). */
	@media (max-width: 52rem) {
		button:not(.clear) span {
			display: none;
		}
	}
</style>
