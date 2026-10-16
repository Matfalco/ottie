<script lang="ts">
	import type { Job } from '@ottie/api-types/Job';
	import type { JobChange } from '@ottie/api-types/JobChange';
	import type { View } from '@ottie/api-types/View';
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
	import { pauseSteps, resumeTargets } from '#lib/bulk.js';
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
	import { currentSpeed } from '#lib/jobs.js';
	import { openProblem, toOpen } from '#lib/opening.js';
	import { m } from '#lib/paraglide/messages.js';
	import { canReorder, type Reorder } from '#lib/queue.js';
	import { NOTHING, keepShown, selectAll, type Selection } from '#lib/selection.js';
	import { isFinished } from '#lib/status.js';
	import { viewBody } from '#lib/views.js';
	import { canOpenFiles, openFile, showInFolder } from '#lib/window.js';
	import AddCard from './AddCard.svelte';
	import ConfirmDialog from './ConfirmDialog.svelte';
	import FilterPanel from './FilterPanel.svelte';
	import DetailsEmpty from './DetailsEmpty.svelte';
	import JobDetails from './JobDetails.svelte';
	import JobTable from './JobTable.svelte';
	import QuickFilters from './QuickFilters.svelte';
	import Toolbar from './Toolbar.svelte';
	import ViewDialog from './ViewDialog.svelte';

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
	/** The view being named: a new one (null) or one to rename. */
	let naming: { view: View | null } | null = $state(null);
	let deleting: View | null = $state(null);

	let today = $derived(new Date(session.now));
	let shown = $derived(session.list.filter((job) => matches(job, filters, today)));
	let sections = $derived(arrange(shown, arrangement));
	let order = $derived(sections.flatMap((section) => section.jobs.map((job) => job.id)));
	// Downloads that were removed or left the view drop out of the selection.
	let current = $derived(keepShown(selection, order));
	let selected = $derived(shown.filter((job) => current.ids.has(job.id)));
	let finished = $derived(shown.filter(isFinished));
	// Against every download, as the daemon moves them, not only those shown.
	let canUp = $derived(canReorder(session.list, selected, 'up'));
	let canDown = $derived(canReorder(session.list, selected, 'down'));
	// The panel under the list shows the one selected download.
	let picked = $derived(selected.length === 1 ? selected[0] : null);
	let detailsOpen = $state(true);
	// Categories with downloads get a quick filter.
	let used = $derived(
		session.categoryList.filter((category) =>
			session.list.some((job) => job.category === category.id)
		)
	);
	// Pause all and Resume all act on every download, not only those shown.
	let steps = $derived(pauseSteps(session.list));
	let resumable = $derived(resumeTargets(session.list));
	/** In the desktop app, a download's file and folder open (docs/decisions/0045). */
	const opens = canOpenFiles();
	let facts = $derived({
		speed: session.history.at(-1) ?? 0,
		running: session.list.filter(isActive).length,
		slots: session.settings?.maxConcurrent ?? null
	});

	function setFilters(next: Filters) {
		filters = next;
		void goto(downloads + filtersQuery(next), { replace: true, reset: false });
	}

	/** Saves a change to one download's own settings; shows why if it can't. */
	async function changeJob(id: string, change: JobChange): Promise<boolean> {
		const why = await session.changeJob(id, change);
		if (why !== null) {
			session.error = why;
		}
		return why === null;
	}

	function setArrangement(next: Arrangement) {
		arrangement = next;
		saveArrangement(next);
	}

	/** Moves the selection in the queue (the waiting group shows it). */
	function reorder(to: Reorder) {
		void session.reorder(
			selected.map((job) => job.id),
			to
		);
	}

	/** Saves what's shown as a new view, or renames one (keeping what it shows). */
	async function saveView(view: View | null, name: string): Promise<string | null> {
		if (view !== null) {
			const { filters, arrangement } = view;
			return session.saveView(view.id, { name: name.trim(), filters, arrangement });
		}
		const why = await session.saveView(null, viewBody(name, filters, arrangement));
		if (why === null) {
			open = false; // Back to the pills, where the new one shows.
		}
		return why;
	}

	/** Opens a finished download's file, or says why it can't. */
	async function openJob(job: Job) {
		const why = await openFile(job.id);
		if (why !== null) {
			session.error = openProblem(why, job, false);
		}
	}

	/** Opens a download's folder with its file selected. */
	async function reveal(job: Job) {
		const why = await showInFolder(job.id);
		if (why !== null) {
			session.error = openProblem(why, job, true);
		}
	}

	/** The waiting downloads first, so pausing a running one doesn't start them. */
	async function pauseAll() {
		await session.run('pause', steps.waiting);
		await session.run('pause', steps.running);
	}

	/** File-manager keys: Ctrl+A, Escape, Delete, Enter (opens the one
	 * finished download selected). Not while typing or while a dialog is open. */
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
		} else if (
			event.key === 'Enter' &&
			opens &&
			!(target instanceof HTMLButtonElement || target instanceof HTMLAnchorElement)
		) {
			const job = toOpen(selected);
			if (job) {
				event.preventDefault();
				void openJob(job);
			}
		}
	}
</script>

<svelte:window onkeydown={keydown} />

<div class="page">
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
				onReorder={reorder}
				{canUp}
				{canDown}
				shown={shown.length}
				total={session.list.length}
				active={activeCount(filters)}
				{open}
				onToggle={() => (open = !open)}
				canPauseAll={steps.waiting.length + steps.running.length > 0}
				canResumeAll={resumable.length > 0}
				onPauseAll={() => void pauseAll()}
				onResumeAll={() => void session.run('resume', resumable)}
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
						onSaveView={() => (naming = { view: null })}
						onClose={() => (open = false)}
					/>
				{/key}
			{:else}
				<QuickFilters
					{filters}
					{arrangement}
					jobs={session.list}
					categories={used}
					views={session.viewList}
					now={today}
					onArrange={setArrangement}
					onRename={(view) => (naming = { view })}
					onDelete={(view) => (deleting = view)}
				/>
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
				onOpen={opens ? openJob : null}
				onSort={(sort) => setArrangement(sortBy(arrangement, sort))}
				onReorder={(ids, to) => void session.reorder(ids, to)}
			/>
		{/if}

		{#if picked}
			{@const job = picked}
			<JobDetails
				{job}
				categories={session.categoryList}
				speed={currentSpeed(session.rates[job.id], session.now)}
				now={session.now}
				maxRetries={session.settings?.maxRetries ?? null}
				open={detailsOpen}
				onToggle={() => (detailsOpen = !detailsOpen)}
				onRun={(action) => run(action, [job])}
				onMove={(category) => session.move([job.id], category)}
				onChange={(change) => changeJob(job.id, change)}
				getLink={() => session.streamLink(job.id)}
				onOpen={opens ? () => void openJob(job) : null}
				onShowFolder={opens ? () => void reveal(job) : null}
			/>
		{:else}
			<DetailsEmpty
				count={selected.length}
				open={detailsOpen}
				onToggle={() => (detailsOpen = !detailsOpen)}
			/>
		{/if}
	</section>
</div>

{#if naming}
	{@const view = naming.view}
	<ViewDialog
		title={view ? m.view_rename_title() : m.view_save_title()}
		name={view?.name ?? ''}
		hint={view ? undefined : m.view_save_hint()}
		onSave={(name) => saveView(view, name)}
		onClose={() => (naming = null)}
	/>
{/if}

{#if deleting}
	{@const view = deleting}
	<ConfirmDialog
		message={m.view_delete_confirm({ name: view.name })}
		confirmLabel={m.view_delete_button()}
		onAnswer={(yes) => {
			deleting = null;
			if (yes) {
				void session.removeView(view.id);
			}
		}}
	/>
{/if}

<style>
	/* The list fills what's left of the page and scrolls inside it. */
	.page {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-height: 0;
	}

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
		border: 1px solid var(--edge);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	.head {
		display: grid;
		gap: 0.5rem;
		padding: 0.6rem 0.75rem 0.75rem;
		border-bottom: 1px solid var(--border);
	}

	/* Fills the room the list would take, so the details stay at the
	   bottom whatever's shown above them. */
	.empty {
		flex: 1;
		margin: 4rem 0;
		padding: 0 1rem;
		text-align: center;
		color: var(--muted);
	}

	div.empty {
		display: grid;
		align-content: start;
		justify-items: center;
		gap: 0.75rem;
	}

	div.empty p {
		margin: 0;
	}

	/* Short windows and phones: the list keeps a height of its own (see
	   JobTable) and the details come under it; the page scrolls. */
	@media (max-height: 40rem), (max-width: 40rem) {
		.page {
			overflow: auto;
		}

		.pane {
			flex: none;
		}
	}

	@media (max-width: 40rem) {
		.add {
			padding: 0.25rem 0.5rem 0.5rem;
		}

		.pane {
			margin: 0 0.5rem 0.5rem;
		}
	}

	/* Rounded (docs/decisions/0044): no line under the filters, the rows
	   carry their own shapes. */
	:global([data-style='rounded']) .head {
		border-bottom: none;
	}
</style>
