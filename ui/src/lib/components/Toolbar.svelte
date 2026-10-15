<script lang="ts">
	import type { Component } from 'svelte';
	import type { Category } from '@ottie/api-types/Category';
	import type { Job } from '@ottie/api-types/Job';
	import Ban from '@lucide/svelte/icons/ban';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import Eraser from '@lucide/svelte/icons/eraser';
	import Funnel from '@lucide/svelte/icons/funnel';
	import Pause from '@lucide/svelte/icons/pause';
	import Play from '@lucide/svelte/icons/play';
	import Trash from '@lucide/svelte/icons/trash';
	import { targets, type Action } from '#lib/bulk.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { Reorder } from '#lib/queue.js';
	import { canFinish } from '#lib/window.js';
	import FinishMenu from './FinishMenu.svelte';
	import MoveMenu from './MoveMenu.svelte';

	interface Props {
		/** The selected downloads. */
		selected: Job[];
		/** Finished downloads in the view, for "Clear finished". */
		finished: Job[];
		onRun: (action: Action, jobs: Job[]) => void;
		categories: Category[];
		/** Moves the selected downloads to that category. */
		onMove: (category: number) => void;
		/** Moves the selected downloads in the queue. */
		onReorder: (to: Reorder) => void;
		/** Whether that would change anything, each way. */
		canUp: boolean;
		canDown: boolean;
		/** Downloads shown, of `total`. */
		shown: number;
		total: number;
		/** How many filters narrow the list. */
		active: number;
		/** Whether "Filters and sorting" is open. */
		open: boolean;
		onToggle: () => void;
	}

	let {
		selected,
		finished,
		onRun,
		categories,
		onMove,
		onReorder,
		canUp,
		canDown,
		shown,
		total,
		active,
		open,
		onToggle
	}: Props = $props();

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
	<MoveMenu {categories} disabled={selected.length === 0} {onMove} />
	<span class="separator" aria-hidden="true"></span>
	<button
		disabled={!canUp}
		onclick={() => onReorder('up')}
		aria-label={m.queue_up()}
		title={m.queue_up()}
	>
		<ChevronUp size={15} aria-hidden="true" />
	</button>
	<button
		disabled={!canDown}
		onclick={() => onReorder('down')}
		aria-label={m.queue_down()}
		title={m.queue_down()}
	>
		<ChevronDown size={15} aria-hidden="true" />
	</button>
	{#if selected.length > 0}
		<span class="count">{m.selected_count({ count: formatCount(selected.length) })}</span>
	{/if}
	<span class="spacer"></span>
	{#if canFinish()}
		<FinishMenu />
	{/if}
	{#if finished.length > 0}
		<button class="clear" onclick={() => onRun('remove', finished)}>
			<Eraser size={15} aria-hidden="true" />
			<span>{m.clear_finished()}</span>
		</button>
	{/if}
	{#if shown < total}
		<span class="shown">
			{m.filters_shown({ shown: formatCount(shown), total: formatCount(total) })}
		</span>
	{/if}
	<button
		class="filters"
		class:open
		aria-expanded={open}
		aria-label={active > 0 ? m.filters_sorting_on({ count: formatCount(active) }) : undefined}
		onclick={onToggle}
	>
		<Funnel size={14} aria-hidden="true" />
		<span>{m.filters_sorting()}</span>
		{#if active > 0}
			<span class="badge" aria-hidden="true">{formatCount(active)}</span>
		{/if}
		{#if open}
			<ChevronUp size={14} aria-hidden="true" />
		{:else}
			<ChevronDown size={14} aria-hidden="true" />
		{/if}
	</button>
</div>

<style>
	.toolbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.25rem;
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
		margin-inline-start: 0.5rem;
		color: var(--muted);
		font-size: 0.9em;
		white-space: nowrap;
	}

	.separator {
		width: 1px;
		height: 1.1rem;
		margin: 0 0.3rem;
		background: var(--border);
	}

	/* Pushes "Clear finished" and the filters to the end. */
	.spacer {
		flex: 1;
	}

	.clear {
		color: var(--muted);
	}

	.shown {
		margin-inline-end: 0.4rem;
		color: var(--faint);
		font-size: 0.85em;
		white-space: nowrap;
	}

	.filters {
		border-color: var(--border);
		font-weight: 500;
	}

	.filters.open,
	.filters.open:hover:not(:disabled) {
		border-color: transparent;
		background: var(--selected);
	}

	.badge {
		min-width: 1.15rem;
		padding: 0 0.3rem;
		border-radius: 999px;
		background: var(--accent);
		color: var(--on-accent);
		font-size: 0.75em;
		line-height: 1.15rem;
		text-align: center;
	}

	/* Narrow: icons only (each button keeps its name for screen readers). */
	@container page (max-width: 56rem) {
		button:not(.clear) span:not(.badge) {
			display: none;
		}
	}
</style>
