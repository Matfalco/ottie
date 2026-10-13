<script lang="ts">
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		arrange,
		groupNote,
		loadArrangement,
		saveArrangement,
		sortBy,
		type Arrangement
	} from '#lib/arrange.js';
	import { useApp } from '#lib/context.js';
	import {
		NO_FILTERS,
		activeCount,
		filtersQuery,
		isActive,
		matches,
		parseFilters,
		type Filters
	} from '#lib/filters.js';
	import { m } from '#lib/paraglide/messages.js';
	import { NOTHING, keepShown, selectAll, type Selection } from '#lib/selection.js';
	import { isFinished } from '#lib/status.js';
	import AddCard from './AddCard.svelte';
	import FilterPanel from './FilterPanel.svelte';
	import JobTable from './JobTable.svelte';
	import QuickFilters from './QuickFilters.svelte';
	import Toolbar from './Toolbar.svelte';

	const { session, run } = useApp();
	const downloads = resolve('/downloads');

	// Read from the address; set at once when changed here, so the next click
	// builds on it even before the address has caught up.
	let filters = $derived(parseFilters(page.url.searchParams));
	let arrangement: Arrangement = $state(loadArrangement());
	let open = $state(false);
	/** Bumped by "Reset filters" under an empty list, so an open panel starts
	 * over (it keeps what's typed in its name field otherwise). */
	let resets = $state(0);
	// Replaced as a whole on every change, never edited in place.
	let selection: Selection = $state.raw(NOTHING);

	let today = $derived(new Date(session.now));
	let shown = $derived(session.list.filter((job) => matches(job, filters, today)));
	let sections = $derived(arrange(shown, arrangement));
	let order = $derived(sections.flatMap((section) => section.jobs.map((job) => job.id)));
	// Downloads that were removed or left the view drop out of the selection.
	let current = $derived(keepShown(selection, order));
	let selected = $derived(shown.filter((job) => current.ids.has(job.id)));
	let finished = $derived(shown.filter(isFinished));
	// Categories with downloads get a quick filter.
	let used = $derived(
		session.categoryList.filter((category) =>
			session.list.some((job) => job.category === category.id)
		)
	);
	let facts = $derived({
		speed: session.history.at(-1) ?? 0,
		running: session.list.filter(isActive).length,
		slots: session.settings?.maxConcurrent ?? null
	});

	function setFilters(next: Filters) {
		filters = next;
		void goto(downloads + filtersQuery(next), { replace: true, reset: false });
	}

	function setArrangement(next: Arrangement) {
		arrangement = next;
		saveArrangement(next);
	}

	/** File-manager keys: Ctrl+A, Escape, Delete. Not while typing or while
	 * a dialog is open. */
	function keydown(event: KeyboardEvent) {
		const target = event.target;
		const typing =
			(target instanceof HTMLInputElement && !['checkbox', 'radio'].includes(target.type)) ||
			target instanceof HTMLTextAreaElement ||
			target instanceof HTMLSelectElement;
		if (typing || document.querySelector('dialog[open]')) {
			return;
		}
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
			event.preventDefault();
			selection = selectAll(order);
		} else if (event.key === 'Escape') {
			selection = NOTHING;
		} else if (event.key === 'Delete' && selected.length > 0) {
			void run('remove', selected);
		}
	}
</script>

<svelte:window onkeydown={keydown} />

<div class="add"><AddCard /></div>

<section class="pane" aria-label={m.nav_downloads()}>
	<div class="head">
		<Toolbar
			{selected}
			{finished}
			onRun={run}
			categories={session.categoryList}
			onMove={(category) =>
				session.move(
					selected.map((job) => job.id),
					category
				)}
			shown={shown.length}
			total={session.list.length}
			active={activeCount(filters)}
			{open}
			onToggle={() => (open = !open)}
		/>
		{#if open}
			{#key resets}
				<FilterPanel
					{filters}
					{arrangement}
					jobs={session.list}
					categories={session.categoryList}
					onFilters={setFilters}
					onArrange={setArrangement}
					onClose={() => (open = false)}
				/>
			{/key}
		{:else}
			<QuickFilters {filters} jobs={session.list} categories={used} now={today} />
		{/if}
	</div>

	{#if session.list.length === 0 && session.connection === 'live'}
		<p class="empty">{m.empty()}</p>
	{:else if shown.length === 0 && session.connection === 'live'}
		<div class="empty">
			<p>{m.empty_filter()}</p>
			<button
				onclick={() => {
					setFilters(NO_FILTERS);
					resets += 1;
				}}>{m.fl_reset_filters()}</button
			>
		</div>
	{:else}
		<JobTable
			{sections}
			{arrangement}
			selection={current}
			categories={session.categories}
			rates={session.rates}
			now={session.now}
			note={(group) => groupNote(group, facts)}
			onSelect={(chosen) => (selection = chosen)}
			onRun={run}
			onSort={(sort) => setArrangement(sortBy(arrangement, sort))}
		/>
	{/if}
</section>

<style>
	.add {
		padding: 0.25rem 1rem 0.75rem;
	}

	/* The quick actions, the filters and the list, on one card that fills
	   the rest of the page; the list scrolls inside it. */
	.pane {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-height: 0;
		margin: 0 1rem 1rem;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	.head {
		display: grid;
		gap: 0.5rem;
		padding: 0.6rem 0.75rem 0.75rem;
		border-bottom: 1px solid var(--border);
	}

	.empty {
		margin: 4rem 0;
		padding: 0 1rem;
		text-align: center;
		color: var(--muted);
	}

	div.empty {
		display: grid;
		justify-items: center;
		gap: 0.75rem;
	}

	div.empty p {
		margin: 0;
	}

	@media (max-width: 40rem) {
		.add {
			padding: 0.25rem 0.5rem 0.5rem;
		}

		.pane {
			margin: 0 0.5rem 0.5rem;
		}
	}
</style>
