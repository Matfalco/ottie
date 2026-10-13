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
		gap: 0.3rem;
		padding: 0.6rem 0.75rem 0;
		overflow-x: auto;
		scrollbar-width: none;
	}

	a {
		display: flex;
		flex: none;
		align-items: center;
		gap: 0.45rem;
		height: 1.9rem;
		padding: 0 0.75rem;
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
		background: var(--selected);
		color: var(--text);
		font-weight: 600;
	}

	.count {
		font-size: 0.85em;
		font-weight: 400;
		color: var(--muted);
	}
</style>
