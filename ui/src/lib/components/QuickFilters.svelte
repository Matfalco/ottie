<script lang="ts">
	import type { Category } from '@ottie/api-types/Category';
	import type { Job } from '@ottie/api-types/Job';
	import { resolve } from '$app/paths';
	import { count, filtersQuery, quickFilters, sameFilters, type Filters } from '#lib/filters.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		filters: Filters;
		/** Every download, for the counts. */
		jobs: Job[];
		/** Categories that have downloads: each gets a pill behind "…". */
		categories: Category[];
		now: Date;
	}

	let { filters, jobs, categories, now }: Props = $props();
	const downloads = resolve('/downloads');

	let pills = $derived(quickFilters(categories));
	let opened = $state(false);
	// A pill behind "…" that's in use keeps them shown (and "Less" hidden).
	let held = $derived(pills.more.some((pill) => sameFilters(pill.filters, filters)));
	let more = $derived(opened || held);
	let shown = $derived(more ? [...pills.main, ...pills.more] : pills.main);
</script>

<!-- Each pill is a link, so it can be bookmarked; switching replaces the
     history entry. -->
<nav class="pills" aria-label={m.filters_label()}>
	{#each shown as pill (pill.key)}
		{@const current = sameFilters(pill.filters, filters)}
		<a
			href={downloads + filtersQuery(pill.filters)}
			class:current
			aria-current={current ? 'true' : undefined}
			data-sveltekit-replacestate
			data-sveltekit-noscroll
			data-sveltekit-keepfocus
		>
			<span>{pill.name}</span>
			<span class="count">{formatCount(count(jobs, pill.filters, now))}</span>
		</a>
	{/each}
	{#if !held}
		<button
			class="more"
			aria-expanded={more}
			aria-label={more ? m.filters_fewer() : m.filters_more()}
			title={more ? m.filters_fewer() : m.filters_more()}
			onclick={() => (opened = !opened)}
		>
			{more ? m.filters_less() : '…'}
		</button>
	{/if}
</nav>

<style>
	.pills {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
	}

	a,
	.more {
		display: flex;
		flex: none;
		align-items: center;
		gap: 0.45rem;
		height: 1.85rem;
		padding: 0 0.8rem;
		border: 1px solid var(--border);
		border-radius: 999px;
		color: var(--text);
		font-size: 0.9em;
		font-weight: 500;
		text-decoration: none;
	}

	a:hover {
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
		opacity: 0.75;
	}

	.more {
		min-width: 2.5rem;
		justify-content: center;
		border-style: dashed;
		background: none;
		color: var(--muted);
		font-weight: 600;
	}

	.more:hover {
		border-color: var(--muted);
		background: var(--hover);
	}
</style>
