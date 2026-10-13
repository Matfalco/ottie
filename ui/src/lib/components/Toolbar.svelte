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
	}

	let { selected, finished, onRun, categories, onMove }: Props = $props();

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
	<!-- Not in the daemon yet (docs/decisions/0021): shown, but disabled. -->
	<button disabled aria-label={m.queue_up()} title={m.coming_later()}>
		<ChevronUp size={15} aria-hidden="true" />
	</button>
	<button disabled aria-label={m.queue_down()} title={m.coming_later()}>
		<ChevronDown size={15} aria-hidden="true" />
	</button>
	{#if selected.length > 0}
		<span class="count">{m.selected_count({ count: formatCount(selected.length) })}</span>
	{/if}
	<span class="spacer"></span>
	{#if finished.length > 0}
		<button class="clear" onclick={() => onRun('remove', finished)}>
			<Eraser size={15} aria-hidden="true" />
			<span>{m.clear_finished()}</span>
		</button>
	{/if}
	<button class="filters" disabled title={m.coming_later()}>
		<Funnel size={14} aria-hidden="true" />
		<span>{m.filters_sorting()}</span>
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
		margin-left: 0.5rem;
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

	/* Pushes "Clear finished" and the filters to the right. */
	.spacer {
		flex: 1;
	}

	.clear {
		color: var(--muted);
	}

	.filters {
		border-color: var(--border);
	}

	/* Narrow: icons only (each button keeps its name for screen readers). */
	@container page (max-width: 56rem) {
		button:not(.clear) span {
			display: none;
		}
	}
</style>
