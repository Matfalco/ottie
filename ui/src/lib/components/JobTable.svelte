<script lang="ts">
	import type { Job } from '@ottie/api-types/Job';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import type { Arrangement, Section, SortKey } from '#lib/arrange.js';
	import type { Action } from '#lib/bulk.js';
	import { categoryName, type Categories } from '#lib/categories.js';
	import { groupName, type Group } from '#lib/filters.js';
	import { formatCount } from '#lib/format.js';
	import { currentSpeed, type Rate } from '#lib/jobs.js';
	import { canOpen } from '#lib/opening.js';
	import { m } from '#lib/paraglide/messages.js';
	import { canDrop, carried, type Reorder } from '#lib/queue.js';
	import { NOTHING, click, selectAll, type Selection } from '#lib/selection.js';
	import JobRow from './JobRow.svelte';

	interface Props {
		/** As shown, top to bottom. */
		sections: Section[];
		arrangement: Arrangement;
		selection: Selection;
		categories: Categories;
		rates: Record<string, Rate>;
		now: number;
		/** What a group's header says at its end. */
		note: (group: Group) => string;
		onSelect: (selection: Selection) => void;
		onRun: (action: Action, jobs: Job[]) => void;
		/** Opens a finished download's file (a double-click); null where files
		 * can't open (a browser). */
		onOpen: ((job: Job) => void) | null;
		/** A click on a sortable column's header. */
		onSort: (sort: SortKey) => void;
		/** Moves downloads in the queue (dragged, or the arrow keys on a grip). */
		onReorder: (ids: string[], to: Reorder) => void;
	}

	let {
		sections,
		arrangement,
		selection,
		categories,
		rates,
		now,
		note,
		onSelect,
		onRun,
		onOpen,
		onSort,
		onReorder
	}: Props = $props();
	let rows = $derived(sections.flatMap((section) => section.jobs));
	let order = $derived(rows.map((job) => job.id));
	let byId = $derived(new Map(rows.map((job) => [job.id, job])));
	let all = $derived(order.length > 0 && selection.ids.size === order.length);
	let some = $derived(selection.ids.size > 0 && !all);

	/** A drag in the queue under way: what it carries, and where it would
	 * drop (next to a waiting download of the same priority). */
	let drag: { jobs: Job[]; over: { id: string; after: boolean } | null } | null = $state.raw(null);
	let scroller: HTMLDivElement | undefined = $state();
	/** Near the list's top or bottom edge, a drag scrolls it. */
	const EDGE = 40;

	function grab(job: Job, event: PointerEvent) {
		if (event.button !== 0) {
			return;
		}
		event.preventDefault();
		const selected = rows.filter((row) => selection.ids.has(row.id));
		drag = { jobs: carried(job, selected), over: null };
	}

	function move(event: PointerEvent) {
		if (drag === null) {
			return;
		}
		if (scroller) {
			const box = scroller.getBoundingClientRect();
			if (event.clientY < box.top + EDGE) {
				scroller.scrollBy(0, -12);
			} else if (event.clientY > box.bottom - EDGE) {
				scroller.scrollBy(0, 12);
			}
		}
		const row = document
			.elementFromPoint(event.clientX, event.clientY)
			?.closest<HTMLElement>('tr[data-id]');
		const target = row ? byId.get(row.dataset.id ?? '') : undefined;
		if (!row || target === undefined || !canDrop(drag.jobs, target)) {
			drag = { ...drag, over: null };
			return;
		}
		const box = row.getBoundingClientRect();
		const after = event.clientY > box.top + box.height / 2;
		drag = { ...drag, over: { id: target.id, after } };
	}

	function drop() {
		if (drag?.over) {
			const { id, after } = drag.over;
			const ids = drag.jobs.map((job) => job.id);
			onReorder(ids, after ? { after: id } : { before: id });
		}
		drag = null;
	}

	function landing(id: string): 'before' | 'after' | null {
		if (drag?.over?.id !== id) {
			return null;
		}
		return drag.over.after ? 'after' : 'before';
	}

	function sorted(sort: SortKey): 'ascending' | 'descending' | undefined {
		if (arrangement.sort !== sort) {
			return undefined;
		}
		return arrangement.direction === 'asc' ? 'ascending' : 'descending';
	}
</script>

{#snippet sortable(sort: SortKey, label: string)}
	<button class="sort" class:on={arrangement.sort === sort} onclick={() => onSort(sort)}>
		{label}
		{#if arrangement.sort === sort}
			{#if arrangement.direction === 'asc'}
				<ArrowUp size={12} aria-hidden="true" />
			{:else}
				<ArrowDown size={12} aria-hidden="true" />
			{/if}
		{/if}
	</button>
{/snippet}

<svelte:window onpointermove={move} onpointerup={drop} onpointercancel={() => (drag = null)} />

<div class="scroller" class:dragging={drag !== null} bind:this={scroller}>
	<table>
		<thead>
			<tr>
				{#if arrangement.grouped}
					<th class="grip"></th>
				{/if}
				<th class="check">
					<input
						type="checkbox"
						checked={all}
						indeterminate={some}
						aria-label={m.select_all()}
						onchange={() => onSelect(all ? NOTHING : selectAll(order))}
					/>
				</th>
				<th class="name" aria-sort={sorted('name')}>{@render sortable('name', m.col_name())}</th>
				<th class="size" aria-sort={sorted('size')}>{@render sortable('size', m.col_size())}</th>
				<th class="progress" aria-sort={sorted('progress')}>
					{@render sortable('progress', m.col_progress())}
				</th>
				<th class="speed">{m.col_speed()}</th>
				<th class="left">{m.col_left()}</th>
				<th class="category">{m.col_category()}</th>
				<th class="added" aria-sort={sorted('added')}>
					{@render sortable('added', m.col_added())}
				</th>
				<th class="actions"></th>
			</tr>
		</thead>
		{#each sections as section (section.group)}
			<tbody>
				{#if section.group !== null}
					<tr class="group {section.group}">
						<th colspan="10" scope="rowgroup">
							<span class="heading">
								<span class="dot" aria-hidden="true"></span>
								<span class="title">{groupName(section.group)}</span>
								<span class="count">{formatCount(section.jobs.length)}</span>
								<span class="note">{note(section.group)}</span>
							</span>
						</th>
					</tr>
				{/if}
				{#each section.jobs as job (job.id)}
					<JobRow
						{job}
						selected={selection.ids.has(job.id)}
						category={categories[job.category] ? categoryName(categories[job.category]) : ''}
						speed={currentSpeed(rates[job.id], now)}
						{now}
						onClick={(keys) => onSelect(click(selection, job.id, keys, order))}
						onRun={(action) => onRun(action, [job])}
						onOpen={onOpen && canOpen(job) ? () => onOpen(job) : null}
						grip={arrangement.grouped}
						movable={section.group === 'waiting'}
						carried={drag?.jobs.some((each) => each.id === job.id) ?? false}
						drop={landing(job.id)}
						onGrab={(event) => grab(job, event)}
						onStep={(to) => onReorder([job.id], to)}
					/>
				{/each}
			</tbody>
		{/each}
	</table>
</div>

<style>
	/* Room for a few rows whatever's under it: the details give way. */
	.scroller {
		flex: 1;
		min-height: 11rem;
		overflow: auto;
	}

	/* Short windows and phones: a height of its own, the details under it
	   (see Downloads). */
	@media (max-height: 40rem), (max-width: 40rem) {
		.scroller {
			flex: none;
			height: 60dvh;
			min-height: 12rem;
		}
	}

	.scroller.dragging,
	.scroller.dragging :global(*) {
		cursor: grabbing;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		/* Column widths come from the header, so they don't jump as numbers change. */
		table-layout: fixed;
	}

	th {
		position: sticky;
		top: 0;
		z-index: 1;
		height: 2.2rem;
		padding: 0 0.5rem;
		background: var(--surface);
		border-bottom: 1px solid var(--border);
		color: var(--faint);
		font-size: 0.8rem;
		font-weight: 600;
		text-align: start;
		white-space: nowrap;
	}

	.sort {
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
		padding: 0;
		border: none;
		background: none;
		color: inherit;
		font-weight: inherit;
	}

	.sort:hover,
	.sort.on {
		color: var(--text);
	}

	.group th {
		position: static;
		height: 2rem;
		background: var(--well);
		border-bottom: 1px solid var(--border-soft);
		color: var(--muted);
	}

	/* The title, count and note on one baseline (the mono ones would sit
	   higher if centred); the dot centred. */
	.heading {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
	}

	.heading .dot {
		align-self: center;
	}

	.dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 2px;
		background: var(--quiet);
	}

	.running .dot {
		background: var(--progress);
	}

	.running .title {
		color: var(--text);
	}

	.attention .dot {
		background: var(--danger);
	}

	.attention .title {
		color: var(--danger);
	}

	.finished .dot {
		background: var(--ok);
	}

	.count,
	.note {
		color: var(--faint);
		font-family: var(--mono);
		font-size: 0.95em;
		font-weight: 400;
	}

	.note {
		margin-inline-start: auto;
	}

	.grip {
		width: 1.6rem;
		padding: 0;
	}

	.check {
		width: 2.4rem;
		padding-inline-end: 0;
		text-align: center;
	}

	.check input {
		margin: 0;
		vertical-align: middle;
	}

	.size {
		width: 6rem;
		text-align: end;
	}

	.progress {
		width: 11rem;
	}

	.speed,
	.left {
		width: 6.5rem;
		text-align: end;
	}

	.category {
		width: 8rem;
	}

	.added {
		width: 10.5rem;
		text-align: end;
	}

	/* "15/10/2026، 10:36:53 ص" is a little longer. */
	:global(:root:lang(ar)) .added {
		width: 11rem;
	}

	.actions {
		width: 4.6rem;
	}

	/* Less room: fewer columns, narrower ones, so names keep about 15rem. A
	   column left out stays at no width rather than going: the group rows
	   span every column, and a column they span but no header holds would
	   take a share of the room. */
	@container page (max-width: 72rem) {
		.category {
			width: 0;
			padding: 0;
			overflow: hidden;
			visibility: hidden;
			white-space: nowrap;
		}
	}

	@container page (max-width: 63rem) {
		.added {
			width: 0;
			padding: 0;
			overflow: hidden;
			visibility: hidden;
			white-space: nowrap;
		}
	}

	@container page (max-width: 53rem) {
		.size {
			width: 0;
			padding: 0;
			overflow: hidden;
			visibility: hidden;
			white-space: nowrap;
		}

		.progress {
			width: 7rem;
		}

		.speed,
		.left {
			width: 5rem;
		}
	}

	/* Phones: rows become blocks (see JobRow); no header. */
	@media (max-width: 40rem) {
		table,
		tbody {
			display: block;
		}

		thead {
			display: none;
		}

		.group,
		.group th {
			display: block;
		}

		.group th {
			padding-top: 0.45rem;
		}
	}

	/* Rounded (docs/decisions/0044): rows apart from each other (see
	   JobRow), each group's title a tinted pill. */
	:global([data-style='rounded']) table {
		border-collapse: separate;
		border-spacing: 0 2px;
	}

	:global([data-style='rounded']) th {
		border-bottom: none;
	}

	:global([data-style='rounded']) .group th {
		height: auto;
		padding: 0.5rem 0.25rem 0.15rem;
		border-bottom: none;
		background: none;
	}

	:global([data-style='rounded']) .heading {
		--hue: var(--quiet);
		padding: 0.3rem 0.8rem;
		border-radius: 999px;
		background: color-mix(in srgb, var(--hue) 12%, transparent);
	}

	:global([data-style='rounded']) .running .heading {
		--hue: var(--progress);
	}

	:global([data-style='rounded']) .attention .heading {
		--hue: var(--danger);
	}

	:global([data-style='rounded']) .finished .heading {
		--hue: var(--ok);
	}

	:global([data-style='rounded']) .dot {
		border-radius: 50%;
		background: var(--hue);
	}
</style>
