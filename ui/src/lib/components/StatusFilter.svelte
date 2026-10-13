<script lang="ts">
	import { resolve } from '$app/paths';
	import { FILTERS, filterName, filterQuery, type Filter } from '#lib/filters.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		filter: Filter;
		counts: Record<Filter, number>;
	}

	let { filter, counts }: Props = $props();
	const downloads = resolve('/downloads');
</script>

<!-- Each status is a link (`?status=…`), so it can be bookmarked or opened
     from the dashboard; switching replaces the history entry. -->
<nav class="filter" aria-label={m.filters_label()}>
	{#each FILTERS as each (each)}
		<a
			href={downloads + filterQuery(each)}
			class:current={each === filter}
			aria-current={each === filter ? 'true' : undefined}
			data-sveltekit-replacestate
			data-sveltekit-noscroll
			data-sveltekit-keepfocus
		>
			<span>{filterName(each)}</span>
			<span class="count">{formatCount(counts[each])}</span>
		</a>
	{/each}
</nav>

<style>
	.filter {
		display: flex;
		gap: 0.35rem;
		overflow-x: auto;
		scrollbar-width: none;
	}

	a {
		display: flex;
		flex: none;
		align-items: center;
		gap: 0.45rem;
		height: 1.85rem;
		padding: 0 0.8rem;
		border: 1px solid var(--border);
		border-radius: 999px;
		color: var(--muted);
		font-size: 0.9em;
		text-decoration: none;
	}

	a:hover {
		color: var(--text);
		background: var(--hover);
	}

	a.current {
		border-color: transparent;
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}

	.count {
		font-family: var(--mono);
		font-size: 0.8em;
		font-weight: 400;
		opacity: 0.8;
	}
</style>
