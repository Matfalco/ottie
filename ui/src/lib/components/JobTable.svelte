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
	import { m } from '#lib/paraglide/messages.js';
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
		/** What a group's header says on its right. */
		note: (group: Group) => string;
		onSelect: (selection: Selection) => void;
		onRun: (action: Action, jobs: Job[]) => void;
		/** A click on a sortable column's header. */
		onSort: (sort: SortKey) => void;
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
		onSort
	}: Props = $props();
	let order = $derived(sections.flatMap((section) => section.jobs.map((job) => job.id)));
	let all = $derived(order.length > 0 && selection.ids.size === order.length);
	let some = $derived(selection.ids.size > 0 && !all);

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

<div class="scroller">
	<table>
		<thead>
			<tr>
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
						<th colspan="9" scope="rowgroup">
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
					/>
				{/each}
			</tbody>
		{/each}
	</table>
</div>

<style>
	.scroller {
		flex: 1;
		min-height: 0;
		overflow: auto;
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
		text-align: left;
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

	.heading {
		display: flex;
		align-items: center;
		gap: 0.5rem;
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
		margin-left: auto;
	}

	.check {
		width: 2.4rem;
		padding-right: 0;
		text-align: center;
	}

	.check input {
		margin: 0;
		vertical-align: middle;
	}

	.size {
		width: 6rem;
		text-align: right;
	}

	.progress {
		width: 11rem;
	}

	.speed,
	.left {
		width: 6.5rem;
		text-align: right;
	}

	.category {
		width: 8rem;
	}

	.added {
		width: 6.5rem;
		text-align: right;
	}

	.actions {
		width: 4.6rem;
	}

	/* Less room beside the menu: fewer columns. */
	@container page (max-width: 60rem) {
		.category {
			display: none;
		}
	}

	@container page (max-width: 52rem) {
		.added {
			display: none;
		}
	}

	@container page (max-width: 42rem) {
		.size {
			display: none;
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
</style>
