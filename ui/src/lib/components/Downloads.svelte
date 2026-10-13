<script lang="ts">
	import { page } from '$app/state';
	import { useApp } from '#lib/context.js';
	import { countByFilter, matches, parseFilter } from '#lib/filters.js';
	import { m } from '#lib/paraglide/messages.js';
	import { NOTHING, keepShown, selectAll, type Selection } from '#lib/selection.js';
	import { isFinished } from '#lib/status.js';
	import AddCard from './AddCard.svelte';
	import JobTable from './JobTable.svelte';
	import StatusFilter from './StatusFilter.svelte';
	import Toolbar from './Toolbar.svelte';

	const { session, run } = useApp();

	let filter = $derived(parseFilter(page.url.searchParams.get('status')));
	// Replaced as a whole on every change, never edited in place.
	let selection: Selection = $state.raw(NOTHING);

	let counts = $derived(countByFilter(session.list));
	let shown = $derived(session.list.filter((job) => matches(job, filter)));
	let order = $derived(shown.map((job) => job.id));
	// Downloads that were removed or left the view drop out of the selection.
	let current = $derived(keepShown(selection, order));
	let selected = $derived(shown.filter((job) => current.ids.has(job.id)));
	let finished = $derived(shown.filter(isFinished));

	/** File-manager keys: Ctrl+A, Escape, Delete. Not while typing or while
	 * a dialog is open. */
	function keydown(event: KeyboardEvent) {
		const target = event.target;
		const typing =
			(target instanceof HTMLInputElement && target.type !== 'checkbox') ||
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
		/>
		<StatusFilter {filter} {counts} />
	</div>

	{#if session.list.length === 0 && session.connection === 'live'}
		<p class="empty">{m.empty()}</p>
	{:else if shown.length === 0 && session.connection === 'live'}
		<p class="empty">{m.empty_filter()}</p>
	{:else}
		<JobTable
			jobs={shown}
			selection={current}
			categories={session.categories}
			rates={session.rates}
			now={session.now}
			onSelect={(chosen) => (selection = chosen)}
			onRun={run}
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

	@media (max-width: 40rem) {
		.add {
			padding: 0.25rem 0.5rem 0.5rem;
		}

		.pane {
			margin: 0 0.5rem 0.5rem;
		}
	}
</style>
