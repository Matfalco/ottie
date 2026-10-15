<script lang="ts">
	import type { Category } from '@ottie/api-types/Category';
	import type { Job } from '@ottie/api-types/Job';
	import type { View } from '@ottie/api-types/View';
	import Bookmark from '@lucide/svelte/icons/bookmark';
	import Ellipsis from '@lucide/svelte/icons/ellipsis';
	import { resolve } from '$app/paths';
	import type { Arrangement } from '#lib/arrange.js';
	import { count, filtersQuery, quickFilters, sameFilters, type Filters } from '#lib/filters.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { showsView, viewFilters } from '#lib/views.js';

	interface Props {
		filters: Filters;
		arrangement: Arrangement;
		/** Every download, for the counts. */
		jobs: Job[];
		/** Categories that have downloads: each gets a pill behind "…". */
		categories: Category[];
		/** Saved views: each gets a pill after the fixed ones. */
		views: View[];
		now: Date;
		/** Picking a view also lays the list out its way. */
		onArrange: (arrangement: Arrangement) => void;
		onRename: (view: View) => void;
		onDelete: (view: View) => void;
	}

	let { filters, arrangement, jobs, categories, views, now, onArrange, onRename, onDelete }: Props =
		$props();
	const downloads = resolve('/downloads');

	let pills = $derived(quickFilters(categories));
	let opened = $state(false);
	// A pill behind "…" that's in use keeps them shown (and "Less" hidden).
	let held = $derived(pills.more.some((pill) => sameFilters(pill.filters, filters)));
	let more = $derived(opened || held);
	let shown = $derived(more ? [...pills.main, ...pills.more] : pills.main);
	/** The view whose menu is open. */
	let menu: number | null = $state(null);

	/** Closes the menu once focus leaves it. */
	function focusOut(event: FocusEvent & { currentTarget: HTMLElement }) {
		const next = event.relatedTarget;
		if (!(next instanceof Node && event.currentTarget.contains(next))) {
			menu = null;
		}
	}

	function choose(action: (view: View) => void, view: View) {
		menu = null;
		action(view);
	}
</script>

<!-- Each pill is a link, so it can be bookmarked; switching replaces the
     history entry. -->
<nav class="pills" aria-label={m.filters_label()}>
	{#each pills.main as pill (pill.key)}
		{@render link(pill.filters, pill.name, sameFilters(pill.filters, filters))}
	{/each}
	{#each views as view (view.id)}
		{@const current = showsView(view, filters, arrangement)}
		<div class="view" class:current onfocusout={focusOut}>
			{@render link(viewFilters(view), view.name, current, view)}
			{#if current}
				<button
					class="options"
					aria-haspopup="menu"
					aria-expanded={menu === view.id}
					aria-label={m.view_options({ name: view.name })}
					title={m.view_options({ name: view.name })}
					onclick={() => (menu = menu === view.id ? null : view.id)}
				>
					<Ellipsis size={15} aria-hidden="true" />
				</button>
				{#if menu === view.id}
					<ul role="menu">
						<li role="none">
							<button role="menuitem" onclick={() => choose(onRename, view)}>
								{m.view_rename()}
							</button>
						</li>
						<li role="none">
							<button role="menuitem" class="danger" onclick={() => choose(onDelete, view)}>
								{m.view_delete()}
							</button>
						</li>
					</ul>
				{/if}
			{/if}
		</div>
	{/each}
	{#each shown.slice(pills.main.length) as pill (pill.key)}
		{@render link(pill.filters, pill.name, sameFilters(pill.filters, filters))}
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

{#snippet link(target: Filters, name: string, current: boolean, view?: View)}
	<a
		href={downloads + filtersQuery(target)}
		class:current
		aria-current={current ? 'true' : undefined}
		onclick={() => view && onArrange(view.arrangement)}
		data-sveltekit-replacestate
		data-sveltekit-noscroll
		data-sveltekit-keepfocus
	>
		{#if view}<Bookmark size={13} aria-hidden="true" />{/if}
		<span dir="auto">{name}</span>
		<span class="count">{formatCount(count(jobs, target, now))}</span>
	</a>
{/snippet}

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

	/* A saved view: its pill, and while it's shown, the button of its menu
	   joined to the pill's right end. */
	.view {
		position: relative;
		display: flex;
	}

	.view a :global(svg) {
		margin-inline-start: -0.2rem;
		opacity: 0.8;
	}

	.view.current a {
		padding-inline-end: 0.55rem;
		border-top-right-radius: 0;
		border-bottom-right-radius: 0;
	}

	.options {
		display: flex;
		align-items: center;
		height: 1.85rem;
		padding: 0 0.5rem 0 0.4rem;
		border: none;
		border-inline-start: 1px solid var(--on-accent);
		border-radius: 0 999px 999px 0;
		background: var(--accent);
		color: var(--on-accent);
	}

	.options:hover {
		background: var(--accent);
		filter: brightness(1.1);
	}

	ul {
		position: absolute;
		top: calc(100% + 0.3rem);
		inset-inline-end: 0;
		z-index: 10;
		min-width: 10rem;
		margin: 0;
		padding: 0.3rem;
		list-style: none;
		border: 1px solid var(--border);
		border-radius: var(--radius-m);
		background: var(--surface);
		box-shadow: var(--shadow);
	}

	ul button {
		width: 100%;
		height: 2.1rem;
		padding: 0 0.6rem;
		border-color: transparent;
		border-radius: var(--radius-s);
		background: none;
		text-align: start;
	}

	ul button:hover {
		border-color: transparent;
		background: var(--hover);
	}

	ul .danger {
		color: var(--danger);
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
