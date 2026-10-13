<script lang="ts">
	import type { Job } from '@ottie/api-types/Job';
	import type { Action } from '#lib/bulk.js';
	import { categoryName, type Categories } from '#lib/categories.js';
	import { currentSpeed, type Rate } from '#lib/jobs.js';
	import { m } from '#lib/paraglide/messages.js';
	import { NOTHING, click, selectAll, type Selection } from '#lib/selection.js';
	import JobRow from './JobRow.svelte';

	interface Props {
		/** As shown, top to bottom. */
		jobs: Job[];
		selection: Selection;
		categories: Categories;
		rates: Record<string, Rate>;
		now: number;
		onSelect: (selection: Selection) => void;
		onRun: (action: Action, jobs: Job[]) => void;
	}

	let { jobs, selection, categories, rates, now, onSelect, onRun }: Props = $props();
	let order = $derived(jobs.map((job) => job.id));
	let all = $derived(jobs.length > 0 && selection.ids.size === jobs.length);
	let some = $derived(selection.ids.size > 0 && !all);
</script>

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
				<th class="name">{m.col_name()}</th>
				<th class="size">{m.col_size()}</th>
				<th class="progress">{m.col_progress()}</th>
				<th class="speed">{m.col_speed()}</th>
				<th class="left">{m.col_left()}</th>
				<th class="category">{m.col_category()}</th>
				<th class="added">{m.col_added()}</th>
				<th class="actions"></th>
			</tr>
		</thead>
		<tbody>
			{#each jobs as job (job.id)}
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
		background: var(--bg);
		border-bottom: 1px solid var(--border);
		color: var(--muted);
		font-size: 0.8rem;
		font-weight: 600;
		text-align: left;
		white-space: nowrap;
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
	}
</style>
