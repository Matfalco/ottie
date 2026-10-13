<script lang="ts">
	import type { Category } from '@ottie/api-types/Category';
	import type { Job } from '@ottie/api-types/Job';
	import type { JobChange } from '@ottie/api-types/JobChange';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import Play from '@lucide/svelte/icons/play';
	import type { Action } from '#lib/bulk.js';
	import { categoryName } from '#lib/categories.js';
	import { PRIORITIES, facts, isPlayable, primaryAction, priorityName } from '#lib/details.js';
	import { explainProblem } from '#lib/explain.js';
	import { connections, fileMap } from '#lib/filemap.js';
	import { statusName, statusOf } from '#lib/filters.js';
	import { displayName, formatPercent } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { amount, fraction } from '#lib/status.js';
	import LimitControl from './LimitControl.svelte';

	interface Props {
		job: Job;
		/** General first, then by name. */
		categories: Category[];
		/** Bytes per second right now; 0 if unknown. */
		speed: number;
		now: number;
		/** The daemon's setting, once known. */
		maxRetries: number | null;
		/** Whether the lower part is shown. */
		open: boolean;
		onToggle: () => void;
		onRun: (action: Action) => void;
		onMove: (category: number) => void;
		/** Saves a change to the job's own settings; resolves to false if it couldn't. */
		onChange: (change: JobChange) => Promise<boolean>;
	}

	let { job, categories, speed, now, maxRetries, open, onToggle, onRun, onMove, onChange }: Props =
		$props();

	/** How many squares the file map has: two rows of 24. */
	const BLOCKS = 48;

	let name = $derived(displayName(job));
	let status = $derived(statusOf(job));
	let primary = $derived(primaryAction(job));
	let done = $derived(fraction(job));
	let map = $derived(fileMap(job, BLOCKS, job.state === 'downloading'));
	let problem = $derived(
		job.state === 'failed' || job.state === 'retrying'
			? explainProblem(job.problem, job.error)
			: null
	);
	// Other is for folders of no category: it's never picked by hand.
	let choices = $derived(
		categories.filter((category) => category.builtIn !== 'other' || category.id === job.category)
	);

	function moved(event: Event & { currentTarget: HTMLSelectElement }) {
		const id = Number(event.currentTarget.value);
		if (id !== job.category) {
			onMove(id);
		}
	}
</script>

<section class="details {job.state}" aria-label={m.det_label()}>
	<div class="top">
		<div class="title">
			<h2 title={name}>{name}</h2>
			<span class="pill {status}">{statusName(status)}</span>
		</div>
		<div class="buttons">
			{#if primary}
				<button class="primary" onclick={() => onRun(primary.action)}>{primary.label}</button>
			{/if}
			<!-- Not in the daemon yet (docs/decisions/0021): shown, but disabled. -->
			{#if isPlayable(job) && job.state !== 'completed'}
				<button disabled title={m.coming_later()}>
					<Play size={13} aria-hidden="true" />
					{m.det_play()}
				</button>
			{/if}
			<button disabled title={m.coming_later()}>{m.det_show_folder()}</button>
			<button class="danger" onclick={() => onRun('remove')}>{m.remove()}</button>
			<button
				class="toggle"
				aria-expanded={open}
				aria-label={open ? m.det_hide() : m.det_show()}
				title={open ? m.det_hide() : m.det_show()}
				onclick={onToggle}
			>
				{#if open}
					<ChevronDown size={16} aria-hidden="true" />
				{:else}
					<ChevronUp size={16} aria-hidden="true" />
				{/if}
			</button>
		</div>
	</div>

	{#if open}
		<div class="body">
			<div class="column">
				<div class="url" title={job.url}>{job.url}</div>
				<div class="amount">
					<span>{amount(job)}</span>
					<span class="muted">{done === null ? '' : formatPercent(done)}</span>
				</div>
				<div class="bar {job.state}" aria-hidden="true">
					<div class="fill" style:width={`${(done ?? 0) * 100}%`}></div>
				</div>
				{#if problem}
					<p class="problem">{problem}</p>
				{/if}
				<dl>
					{#each facts(job, speed, now, maxRetries) as fact (fact.label)}
						<div>
							<dt>{fact.label}</dt>
							<dd>{fact.value}</dd>
						</div>
					{/each}
				</dl>
			</div>

			<div class="column">
				<div class="map-head">
					<span>{m.det_file_map()}</span>
					<span class="mono">{connections(job)}</span>
				</div>
				{#if map}
					<div
						class="blocks"
						role="img"
						aria-label={m.det_map_label({ percent: formatPercent(done ?? 0) })}
					>
						{#each map as block, index (index)}
							<span class={block}></span>
						{/each}
					</div>
					<div class="legend" aria-hidden="true">
						<span><span class="key done"></span>{m.det_map_done()}</span>
						<span><span class="key arriving"></span>{m.det_map_arriving()}</span>
						<span><span class="key"></span>{m.det_map_left()}</span>
					</div>
				{:else}
					<p class="faint">{m.det_map_unknown()}</p>
				{/if}
			</div>

			<div class="settings">
				<label class="field">
					<span class="label">{m.det_category()}</span>
					<select value={job.category} onchange={moved}>
						{#each choices as category (category.id)}
							<option value={category.id}>{categoryName(category)}</option>
						{/each}
					</select>
				</label>
				<div class="field">
					<span class="label">{m.det_folder()}</span>
					<span class="folder" title={job.destDir}>{job.destDir}</span>
				</div>
				<div class="field wide">
					<span class="label">{m.det_speed_limit()}</span>
					<LimitControl
						limit={job.speedLimit}
						label={m.det_limit_switch()}
						memory="download"
						onChange={(speedLimit) => onChange({ speedLimit })}
					/>
					<span class="hint">{m.det_limit_hint()}</span>
				</div>
				<fieldset class="field wide">
					<legend class="label">{m.det_priority()}</legend>
					<div class="segments">
						{#each PRIORITIES as priority (priority)}
							<label class:chosen={job.priority === priority}>
								<input
									type="radio"
									name="priority"
									checked={job.priority === priority}
									onchange={() => onChange({ priority })}
								/>
								{priorityName(priority)}
							</label>
						{/each}
					</div>
					<span class="hint">{m.det_priority_hint()}</span>
				</fieldset>
			</div>
		</div>
	{/if}
</section>

<style>
	/* Under the list, on the same card; scrolls by itself on small screens. */
	.details {
		flex: none;
		max-height: 60%;
		overflow: auto;
		border-top: 1px solid var(--border);
		background: var(--well);
	}

	.top {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
		padding: 0.6rem 0.9rem;
	}

	.title {
		display: flex;
		flex: 1 1 20rem;
		align-items: baseline;
		gap: 0.75rem;
		min-width: 0;
	}

	h2 {
		margin: 0;
		overflow: hidden;
		font-size: 1.05rem;
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.pill {
		flex: none;
		padding: 0.1rem 0.6rem;
		border-radius: 999px;
		background: var(--input);
		color: var(--muted);
		font-size: 0.8rem;
		font-weight: 600;
	}

	.pill.downloading {
		background: var(--selected);
		color: var(--text);
	}

	.pill.retrying {
		background: var(--warn-bg);
		color: var(--warn);
	}

	.pill.failed {
		background: var(--danger-bg);
		color: var(--danger);
	}

	.pill.completed {
		background: var(--ok-bg);
		color: var(--ok);
	}

	.buttons {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
	}

	.buttons button {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		height: 2rem;
		font-weight: 500;
	}

	.danger {
		border-color: transparent;
		background: none;
		color: var(--danger);
	}

	.toggle {
		width: 2rem;
		justify-content: center;
		padding: 0;
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	.body {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(17rem, 1fr));
		gap: 1.25rem 1.5rem;
		padding: 0.25rem 0.9rem 1rem;
	}

	.column {
		display: grid;
		align-content: start;
		gap: 0.6rem;
		min-width: 0;
	}

	.url,
	.folder {
		overflow: hidden;
		color: var(--muted);
		font-family: var(--mono);
		font-size: 0.8rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.amount {
		display: flex;
		justify-content: space-between;
		font-family: var(--mono);
		font-size: 0.85rem;
	}

	.muted {
		color: var(--muted);
	}

	.faint {
		color: var(--faint);
		font-size: 0.85em;
	}

	.bar {
		height: 0.5rem;
		overflow: hidden;
		border-radius: 4px;
		background: var(--track);
	}

	.fill {
		height: 100%;
		background: var(--progress);
		transition: width 0.25s linear;
	}

	.completed .fill {
		background: var(--ok);
	}

	.paused .fill,
	.queued .fill,
	.cancelled .fill {
		background: var(--quiet);
	}

	.retrying .fill {
		background: var(--warn);
	}

	.failed .fill {
		background: var(--danger);
	}

	.problem {
		margin: 0;
		padding: 0.5rem 0.65rem;
		border-radius: var(--radius-s);
		background: var(--danger-bg);
		color: var(--danger);
		font-size: 0.85rem;
	}

	dl {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 0.5rem;
		margin: 0;
	}

	dt {
		color: var(--faint);
		font-size: 0.8rem;
	}

	dd {
		margin: 0.1rem 0 0;
		overflow: hidden;
		font-family: var(--mono);
		font-size: 0.9rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.map-head {
		display: flex;
		justify-content: space-between;
		gap: 0.5rem;
		color: var(--faint);
	}

	.mono {
		font-family: var(--mono);
		font-size: 0.8rem;
	}

	.blocks {
		display: grid;
		grid-template-columns: repeat(24, minmax(0, 1fr));
		gap: 2px;
	}

	.blocks span,
	.key {
		height: 0.7rem;
		border-radius: 1px;
		background: var(--track);
	}

	/* Received blocks take the bar's colour for the state. */
	.blocks .done,
	.blocks .partial,
	.key.done {
		background: var(--progress);
	}

	.blocks .partial {
		opacity: 0.45;
	}

	.blocks .arriving,
	.key.arriving {
		background: var(--arriving);
	}

	.completed .blocks .done {
		background: var(--ok);
	}

	.paused .blocks span:not(.left),
	.queued .blocks span:not(.left),
	.cancelled .blocks span:not(.left) {
		background: var(--quiet);
	}

	.failed .blocks span:not(.left) {
		background: var(--danger);
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem 0.9rem;
		color: var(--faint);
		font-size: 0.8rem;
	}

	.legend > span {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	.key {
		display: inline-block;
		width: 0.55rem;
		height: 0.55rem;
	}

	p.faint {
		margin: 0;
	}

	.settings {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		align-content: start;
		gap: 0.75rem 1rem;
	}

	.field {
		display: grid;
		align-content: start;
		gap: 0.3rem;
		min-width: 0;
		margin: 0;
		padding: 0;
		border: none;
	}

	.wide {
		grid-column: 1 / -1;
	}

	.label {
		padding: 0;
		color: var(--faint);
		font-size: 0.8rem;
	}

	select {
		height: 2rem;
		padding-block: 0;
	}

	.folder {
		display: flex;
		align-items: center;
		height: 2rem;
		padding: 0 0.6rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--input);
	}

	.hint {
		color: var(--faint);
		font-size: 0.8rem;
	}

	/* Radio buttons drawn as one row of segments (as in the filters). */
	.segments {
		display: flex;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--surface);
	}

	.segments label {
		display: grid;
		flex: 1;
		place-items: center;
		min-height: 1.95rem;
		padding: 0 0.4rem;
		color: var(--muted);
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
		font-weight: 500;
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

	@media (max-width: 40rem) {
		dl {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
</style>
