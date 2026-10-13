<script lang="ts">
	import type { Category } from '@ottie/api-types/Category';
	import type { Job } from '@ottie/api-types/Job';
	import { untrack } from 'svelte';
	import {
		DEFAULT_ARRANGEMENT,
		FIRST_DIRECTION,
		SORT_KEYS,
		directionName,
		sortName,
		type Arrangement,
		type Direction
	} from '#lib/arrange.js';
	import { categoryHue, categoryName } from '#lib/categories.js';
	import {
		ADDED,
		NO_FILTERS,
		STATUSES,
		addedName,
		statusName,
		statusOf,
		toggleCategory,
		toggleStatus,
		type Filters
	} from '#lib/filters.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		filters: Filters;
		arrangement: Arrangement;
		/** Every download, for the counts. */
		jobs: Job[];
		categories: Category[];
		onFilters: (filters: Filters) => void;
		onArrange: (arrangement: Arrangement) => void;
		/** Asks for a name to save what's shown as a view. */
		onSaveView: () => void;
		onClose: () => void;
	}

	let { filters, arrangement, jobs, categories, onFilters, onArrange, onSaveView, onClose }: Props =
		$props();

	// Typed here and sent on each key, but not read back from the address
	// while typing: a slow round trip would otherwise undo the last letters.
	let query = $state(untrack(() => filters.query));
	let ids = $derived(categories.map((category) => category.id));
	const DIRECTIONS: Direction[] = ['desc', 'asc'];

	function countWhere(test: (job: Job) => boolean): string {
		return formatCount(jobs.filter(test).length);
	}

	function reset() {
		query = '';
		onFilters(NO_FILTERS);
		onArrange(DEFAULT_ARRANGEMENT);
	}
</script>

<div class="panel">
	<div class="fields">
		<fieldset>
			<legend>{m.fl_status()}</legend>
			{#each STATUSES as status (status)}
				<label class="box">
					<input
						type="checkbox"
						checked={filters.statuses?.includes(status) ?? true}
						onchange={() => onFilters(toggleStatus(filters, status))}
					/>
					<span class="dot status-{status}" aria-hidden="true"></span>
					<span class="name">{statusName(status)}</span>
					<span class="count">{countWhere((job) => statusOf(job) === status)}</span>
				</label>
			{/each}
		</fieldset>

		<fieldset>
			<legend>{m.fl_category()}</legend>
			{#each categories as category (category.id)}
				<label class="box">
					<input
						type="checkbox"
						checked={filters.categories?.includes(category.id) ?? true}
						onchange={() => onFilters(toggleCategory(filters, category.id, ids))}
					/>
					<span class="dot" style:background={categoryHue(category)} aria-hidden="true"></span>
					<span class="name">{categoryName(category)}</span>
					<span class="count">{countWhere((job) => job.category === category.id)}</span>
				</label>
			{/each}
		</fieldset>

		<div class="column">
			<label class="text">
				<span class="heading">{m.fl_name()}</span>
				<input
					type="search"
					bind:value={query}
					oninput={() => onFilters({ ...filters, query })}
					placeholder={m.fl_name_example()}
				/>
			</label>
			<fieldset>
				<legend>{m.fl_added()}</legend>
				{#each ADDED as added (added)}
					<label class="box">
						<input
							type="radio"
							name="added"
							checked={filters.added === added}
							onchange={() => onFilters({ ...filters, added })}
						/>
						<span class="name">{addedName(added)}</span>
					</label>
				{/each}
			</fieldset>
		</div>

		<div class="column">
			<fieldset>
				<legend>{m.fl_arrange()}</legend>
				<label class="box">
					<input
						type="checkbox"
						checked={arrangement.grouped}
						onchange={() => onArrange({ ...arrangement, grouped: !arrangement.grouped })}
					/>
					<span class="name">{m.fl_group()}</span>
				</label>
			</fieldset>
			<fieldset>
				<legend>{m.fl_sort()}</legend>
				<div class="segments">
					{#each SORT_KEYS as sort (sort)}
						<label class:chosen={arrangement.sort === sort}>
							<input
								type="radio"
								name="sort"
								checked={arrangement.sort === sort}
								onchange={() =>
									onArrange({ ...arrangement, sort, direction: FIRST_DIRECTION[sort] })}
							/>
							{sortName(sort)}
						</label>
					{/each}
				</div>
			</fieldset>
			<fieldset>
				<legend class="hidden">{m.fl_order()}</legend>
				<div class="segments">
					{#each DIRECTIONS as direction (direction)}
						<label class:chosen={arrangement.direction === direction}>
							<input
								type="radio"
								name="direction"
								checked={arrangement.direction === direction}
								onchange={() => onArrange({ ...arrangement, direction })}
							/>
							{directionName(arrangement.sort, direction)}
						</label>
					{/each}
				</div>
			</fieldset>
		</div>
	</div>

	<div class="footer">
		<button class="plain" onclick={reset}>{m.fl_reset()}</button>
		<span class="hint">{m.fl_views_hint()}</span>
		<button onclick={onSaveView}>{m.fl_save_view()}</button>
		<button class="primary" onclick={onClose}>{m.fl_done()}</button>
	</div>
</div>

<style>
	.panel {
		display: grid;
		gap: 0.75rem;
		padding: 0.75rem;
		border-radius: var(--radius-m);
		background: var(--well);
	}

	.fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: 1rem 1.5rem;
	}

	fieldset {
		display: grid;
		gap: 0.15rem;
		min-width: 0;
		margin: 0;
		padding: 0;
		border: none;
	}

	.column {
		display: grid;
		align-content: start;
		gap: 0.75rem;
	}

	legend,
	.heading {
		padding: 0 0 0.35rem;
		color: var(--faint);
		font-size: 0.8rem;
		font-weight: 600;
	}

	/* Still read out, but the sort heading above already says it. */
	legend.hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
	}

	.box {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 1.6rem;
		cursor: pointer;
	}

	.box input {
		margin: 0;
		accent-color: var(--accent);
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.dot {
		flex: none;
		width: 0.45rem;
		height: 0.45rem;
		border-radius: 2px;
		background: var(--quiet);
	}

	.status-downloading {
		background: var(--progress);
	}

	.status-retrying {
		background: var(--warn);
	}

	.status-failed {
		background: var(--danger);
	}

	.status-completed {
		background: var(--ok);
	}

	.count {
		color: var(--faint);
		font-family: var(--mono);
		font-size: 0.8em;
	}

	.text {
		display: grid;
	}

	.text input {
		height: 2rem;
		min-width: 0;
	}

	/* Radio buttons drawn as one row of segments. */
	.segments {
		display: flex;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
	}

	.segments label {
		display: grid;
		flex: 1;
		place-items: center;
		min-height: 1.85rem;
		padding: 0 0.4rem;
		color: var(--muted);
		font-weight: 500;
		text-align: center;
		cursor: pointer;
	}

	.segments label + label {
		border-left: 1px solid var(--border);
	}

	.segments label:hover {
		background: var(--hover);
	}

	.segments label.chosen {
		background: var(--selected);
		color: var(--text);
	}

	.segments label:focus-within {
		outline: 2px solid var(--focus);
		outline-offset: -2px;
	}

	.segments input {
		position: absolute;
		opacity: 0;
		pointer-events: none;
	}

	.footer {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
		padding-top: 0.6rem;
		border-top: 1px solid var(--border);
	}

	.plain {
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	.hint {
		margin-left: auto;
		color: var(--faint);
		font-size: 0.85em;
	}
</style>
