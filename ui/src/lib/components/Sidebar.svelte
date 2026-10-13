<script lang="ts">
	import type { Component } from 'svelte';
	import ArrowDownToLine from '@lucide/svelte/icons/arrow-down-to-line';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import CirclePause from '@lucide/svelte/icons/circle-pause';
	import Clock from '@lucide/svelte/icons/clock';
	import List from '@lucide/svelte/icons/list';
	import { FILTERS, filterName, type Filter } from '#lib/filters.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { Connection } from '#lib/session.svelte.js';
	import LanguagePicker from './LanguagePicker.svelte';

	interface Props {
		filter: Filter;
		counts: Record<Filter, number>;
		connection: Connection;
		onSelect: (filter: Filter) => void;
	}

	let { filter, counts, connection, onSelect }: Props = $props();

	const ICONS: Record<Filter, Component> = {
		all: List,
		active: ArrowDownToLine,
		queued: Clock,
		stopped: CirclePause,
		completed: CircleCheck
	};
</script>

<aside>
	<nav aria-label={m.filters_label()}>
		{#each FILTERS as each (each)}
			{@const Icon = ICONS[each]}
			<button
				class:current={each === filter}
				aria-current={each === filter ? 'true' : undefined}
				onclick={() => onSelect(each)}
			>
				<Icon size={16} aria-hidden="true" />
				<span class="name">{filterName(each)}</span>
				<span class="count">{formatCount(counts[each])}</span>
			</button>
		{/each}
	</nav>

	<div class="foot">
		<span class="connection {connection}" role="status">
			{#if connection === 'live'}
				{m.connection_live()}
			{:else if connection === 'connecting'}
				{m.connection_connecting()}
			{:else}
				{m.connection_offline()}
			{/if}
		</span>
		<LanguagePicker />
	</div>
</aside>

<style>
	aside {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		min-height: 0;
		padding: 0.75rem 0.6rem;
		background: var(--chrome);
		border-right: 1px solid var(--border);
	}

	nav {
		display: grid;
		gap: 0.1rem;
	}

	nav button {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		height: 2.1rem;
		padding: 0 0.6rem;
		border-color: transparent;
		background: none;
		color: var(--muted);
		text-align: left;
	}

	nav button:hover {
		border-color: transparent;
		background: var(--hover);
		color: var(--text);
	}

	nav button.current {
		background: var(--selected);
		color: var(--text);
		font-weight: 600;
	}

	nav button.current :global(svg) {
		color: var(--accent);
	}

	.name {
		flex: 1;
	}

	.count {
		font-size: 0.85em;
		font-weight: 400;
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}

	.foot {
		display: grid;
		gap: 0.6rem;
		margin-top: auto;
		padding: 0 0.4rem;
	}

	.connection {
		color: var(--muted);
		font-size: 0.85em;
	}

	.connection::before {
		content: '●';
		margin-right: 0.35em;
	}

	.connection.live::before {
		color: var(--progress);
	}

	.connection.offline {
		color: var(--danger);
	}

	/* Narrow: the views in a row that scrolls sideways, under the title bar. */
	@media (max-width: 40rem) {
		aside {
			display: grid;
			/* minmax(0, …): the row of views scrolls instead of widening the page. */
			grid-template-columns: minmax(0, 1fr) auto;
			align-items: center;
			gap: 0.5rem;
			padding: 0.4rem 0.75rem;
			border-right: none;
			border-bottom: 1px solid var(--border);
		}

		nav {
			display: flex;
			overflow-x: auto;
			scrollbar-width: none;
		}

		nav button {
			flex: none;
		}

		.foot {
			display: flex;
			margin-top: 0;
			padding: 0;
		}

		/* Room is short: say something only when it isn't connected. */
		.connection.live {
			display: none;
		}
	}
</style>
