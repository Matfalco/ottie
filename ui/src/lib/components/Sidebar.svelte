<script lang="ts">
	import icon from '#lib/assets/ottie.svg';
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
</script>

<aside>
	<div class="brand">
		<img src={icon} alt="" width="32" height="32" />
		<h1>Ottie</h1>
	</div>

	<nav aria-label={m.filters_label()}>
		{#each FILTERS as each (each)}
			<button
				class:current={each === filter}
				aria-current={each === filter ? 'true' : undefined}
				onclick={() => onSelect(each)}
			>
				<span>{filterName(each)}</span>
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
		/* Stays in view while the list scrolls. */
		position: sticky;
		top: 0;
		height: 100vh;
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		padding: 1rem 0.75rem;
		background: var(--surface);
		border-right: 1px solid var(--border);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 0.55rem;
		padding: 0 0.5rem;
	}

	.brand img {
		border-radius: 8px;
	}

	h1 {
		margin: 0;
		font-size: 1.25rem;
	}

	nav {
		display: grid;
		gap: 0.15rem;
	}

	nav button {
		display: flex;
		justify-content: space-between;
		gap: 0.75rem;
		border-color: transparent;
		background: none;
		text-align: left;
	}

	nav button:hover {
		border-color: transparent;
		background: var(--track);
	}

	nav button.current {
		background: var(--selected);
		color: var(--accent);
		font-weight: 600;
	}

	.count {
		color: var(--muted);
		font-variant-numeric: tabular-nums;
	}

	.foot {
		display: grid;
		gap: 0.6rem;
		margin-top: auto;
		padding: 0 0.5rem;
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

	/* Phones: a bar on top, the views in a row that scrolls sideways. */
	@media (max-width: 40rem) {
		aside {
			position: static;
			height: auto;
			display: grid;
			/* minmax(0, …): the row of views scrolls instead of widening the page. */
			grid-template-columns: auto minmax(0, 1fr);
			grid-template-areas: 'brand foot' 'nav nav';
			gap: 0.6rem 1rem;
			padding: 0.6rem 1rem;
			border-right: none;
			border-bottom: 1px solid var(--border);
		}

		.brand {
			grid-area: brand;
			padding: 0;
		}

		nav {
			grid-area: nav;
			display: flex;
			overflow-x: auto;
			scrollbar-width: thin;
		}

		nav button {
			flex: none;
		}

		.foot {
			grid-area: foot;
			display: flex;
			align-items: center;
			justify-content: flex-end;
			margin-top: 0;
			padding: 0;
		}
	}
</style>
