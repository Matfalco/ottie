<script lang="ts">
	import { untrack } from 'svelte';
	import type { Category } from '@ottie/api-types/Category';
	import type { Job } from '@ottie/api-types/Job';
	import type { JobChange } from '@ottie/api-types/JobChange';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import Copy from '@lucide/svelte/icons/copy';
	import ListVideo from '@lucide/svelte/icons/list-video';
	import Play from '@lucide/svelte/icons/play';
	import type { Action } from '#lib/bulk.js';
	import { categoryName } from '#lib/categories.js';
	import {
		PRIORITIES,
		canStream,
		facts,
		isPlayable,
		primaryAction,
		priorityName
	} from '#lib/details.js';
	import { explainProblem } from '#lib/explain.js';
	import { connections, strip } from '#lib/filemap.js';
	import { statusName, statusOf } from '#lib/filters.js';
	import { displayName, formatPercent } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { savePlaylist } from '#lib/playlist.js';
	import { canSchedule, fromField, timeChange, toField, type TimeKey } from '#lib/schedule.js';
	import { amount, fraction } from '#lib/status.js';
	import { canChoosePlayer } from '#lib/window.js';
	import LimitControl from './LimitControl.svelte';
	import Switch from './Switch.svelte';
	import { useApp } from '#lib/context.js';

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
		/** The job's play link; null if the daemon can't give one. */
		getLink: () => Promise<string | null>;
	}

	let {
		job,
		categories,
		speed,
		now,
		maxRetries,
		open,
		onToggle,
		onRun,
		onMove,
		onChange,
		getLink
	}: Props = $props();
	const { session } = useApp();

	let name = $derived(displayName(job));
	let status = $derived(statusOf(job));
	let primary = $derived(primaryAction(job));
	let done = $derived(fraction(job));
	// The bar maps the file: each range at its place, once the size is known.
	let parts = $derived(strip(job, job.state === 'downloading'));
	let problem = $derived(
		job.state === 'failed' || job.state === 'retrying'
			? explainProblem(job.problem, job.error)
			: null
	);
	// Other is for folders of no category: it's never picked by hand.
	let choices = $derived(
		categories.filter((category) => category.builtIn !== 'other' || category.id === job.category)
	);

	// The play link, asked for again only when another download is shown or
	// this one becomes playable (not on every change).
	let id = $derived(job.id);
	let playable = $derived(isPlayable(job));
	let link: string | null = $state(null);
	/** In the desktop app, Play opens the player chosen in Settings. */
	const inApp = canChoosePlayer();
	let copied = $state(false);
	/** The clipboard refused: the link is shown to copy by hand. */
	let showLink = $state(false);

	$effect(() => {
		const wanted = playable ? id : null;
		link = null;
		copied = false;
		showLink = false;
		if (wanted === null) {
			return;
		}
		let current = true;
		void untrack(getLink).then((url) => {
			if (current) {
				link = url;
			}
		});
		return () => {
			current = false;
		};
	});

	async function copy(url: string) {
		try {
			await navigator.clipboard.writeText(url);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			showLink = true; // Not allowed here (a plain-http remote page).
		}
	}

	function percent(fraction: number): string {
		return `${fraction * 100}%`;
	}

	/** Sends a time once the field is left (typing a date fires `change` at
	 * each part, which would stop and start a running download). A refused
	 * or unfinished one shows the download's time again. */
	async function commit(input: HTMLInputElement, key: TimeKey, current: number | null) {
		if (input.value === toField(current)) {
			return;
		}
		const ms = fromField(input.value);
		if ((input.value !== '' && ms === null) || !(await onChange(timeChange(key, ms)))) {
			input.value = toField(current);
		}
	}

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
			{#if link}
				<!-- A real link: the browser plays it in a new tab (the desktop
				     app hands it to the player chosen in Settings). -->
				<a
					class="button"
					href={link}
					target="_blank"
					rel="noopener"
					title={inApp ? m.det_play_hint_app() : m.det_play_hint()}
				>
					<Play size={13} aria-hidden="true" />
					{m.det_play()}
				</a>
				{#if !inApp}
					<!-- A web page can't start a player: it saves a playlist
					     that the system opens with one. -->
					<button title={m.det_open_player_hint()} onclick={() => link && savePlaylist(name, link)}>
						<ListVideo size={13} aria-hidden="true" />
						{m.det_open_player()}
					</button>
				{/if}
				<button title={m.det_copy_link_hint()} onclick={() => link && copy(link)}>
					<Copy size={13} aria-hidden="true" />
					{copied ? m.det_link_copied() : m.det_copy_link()}
				</button>
			{/if}
			<button disabled title={m.coming_later()}>{m.det_show_folder()}</button>
			<button class="danger" onclick={() => onRun('remove')}>{m.remove()}</button>
		</div>
	</div>
	<!-- In the top right corner, wherever the buttons wrap to. -->
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

	{#if showLink && link}
		<input class="link" readonly value={link} aria-label={m.det_link_label()} />
	{/if}

	{#if open}
		<div class="body">
			<div class="column">
				<div class="url" title={job.url}>{job.url}</div>
				<div class="amount">
					<span>{amount(job)}</span>
					<span class="muted">{done === null ? '' : formatPercent(done)}</span>
				</div>
				<div
					class="bar"
					role="img"
					aria-label={m.det_map_label({ percent: formatPercent(done ?? 0) })}
				>
					{#if parts}
						{#each parts as part (part.start)}
							<span
								class="range"
								class:writing={part.writing}
								style:left={percent(part.start)}
								style:width={percent(part.end - part.start)}
							>
								<span
									class="fill"
									style:width={percent(
										part.end > part.start
											? (part.reached - part.start) / (part.end - part.start)
											: 1
									)}
								></span>
							</span>
						{/each}
					{:else}
						<span class="range" style:width="100%">
							<span class="fill" style:width={percent(done ?? 0)}></span>
						</span>
					{/if}
				</div>
				<div class="map-foot">
					<span class="mono">{connections(job)}</span>
					{#if parts && parts.length > 1}
						<div class="legend" aria-hidden="true">
							<span><span class="key done"></span>{m.det_map_done()}</span>
							{#if job.state === 'downloading'}
								<span><span class="key arriving"></span>{m.det_map_arriving()}</span>
							{/if}
							<span><span class="key"></span>{m.det_map_left()}</span>
						</div>
					{/if}
				</div>
				{#if problem}
					<p class="problem">{problem}</p>
				{/if}
				<dl>
					{#each facts(job, speed, now, maxRetries, undefined, session.slotNames) as fact (fact.label)}
						<div>
							<dt>{fact.label}</dt>
							<dd>{fact.value}</dd>
						</div>
					{/each}
				</dl>
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
				{#if canStream(job)}
					<div class="field wide">
						<span class="label">{m.det_streaming()}</span>
						<span class="switch-row">
							<Switch
								on={job.streaming}
								label={m.det_streaming()}
								onChange={(streaming) => onChange({ streaming })}
							/>
							<span class="hint">{m.det_streaming_hint()}</span>
						</span>
					</div>
				{/if}
				{#if canSchedule(job)}
					<div class="field wide">
						<span class="label">{m.det_schedule()}</span>
						<div class="times">
							{@render timeField(m.det_start_at(), 'startAt', job.startAt)}
							{@render timeField(m.det_stop_at(), 'stopAt', job.stopAt)}
						</div>
						<span class="hint">{m.det_schedule_hint()}</span>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</section>

{#snippet timeField(label: string, key: TimeKey, value: number | null)}
	<label class="time">
		<span class="sub">{label}</span>
		<span class="time-row">
			<input
				type="datetime-local"
				value={toField(value)}
				onblur={(event) => commit(event.currentTarget, key, value)}
				onkeydown={(event) => event.key === 'Enter' && event.currentTarget.blur()}
			/>
			{#if value !== null}
				<button type="button" onclick={() => onChange(timeChange(key, null))}>
					{m.det_clear_time()}
				</button>
			{/if}
		</span>
	</label>
{/snippet}

<style>
	/* Under the list, on the same card; scrolls by itself on small screens. */
	.details {
		position: relative;
		flex: none;
		max-height: 60%;
		overflow: auto;
		border-top: 1px solid var(--border);
		background: var(--well);
	}

	/* Room on the right for the arrow. */
	.top {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
		padding: 0.6rem 3.4rem 0.6rem 0.9rem;
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

	.buttons button,
	.buttons a {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		height: 2rem;
		font-weight: 500;
	}

	/* A link that looks like the buttons next to it. */
	a.button {
		padding: 0.4rem 0.85rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--surface);
		color: inherit;
		text-decoration: none;
	}

	a.button:hover {
		border-color: var(--muted);
	}

	.link {
		display: block;
		width: calc(100% - 1.8rem);
		margin: 0 0.9rem 0.6rem;
		font-family: var(--mono);
		font-size: 0.8rem;
	}

	.danger {
		border-color: transparent;
		background: none;
		color: var(--danger);
	}

	.toggle {
		position: absolute;
		top: 0.6rem;
		right: 0.9rem;
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
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

	/* The bar maps the file: each range sits at its place, split from the
	   next by a hairline in the panel's colour. */
	.bar {
		position: relative;
		height: 0.85rem;
		overflow: hidden;
		border-radius: 4px;
		background: var(--track);
	}

	.range {
		position: absolute;
		top: 0;
		bottom: 0;
		overflow: hidden;
	}

	.range + .range {
		border-left: 1px solid var(--well);
	}

	.fill {
		position: relative;
		display: block;
		height: 100%;
		background: var(--progress);
		transition: width 0.25s linear;
	}

	/* Where a connection is writing right now. */
	.writing .fill::after {
		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		width: 2px;
		background: var(--arriving);
		content: '';
	}

	.map-foot {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 0.3rem 0.9rem;
		color: var(--faint);
	}

	.completed .fill {
		background: var(--ok);
	}

	.paused .fill,
	.queued .fill,
	.scheduled .fill,
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

	.mono {
		font-family: var(--mono);
		font-size: 0.8rem;
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
		border-radius: 1px;
		background: var(--track);
	}

	/* Received takes the bar's colour for the state. */
	.key.done {
		background: var(--progress);
	}

	.paused .key.done,
	.queued .key.done,
	.scheduled .key.done,
	.cancelled .key.done {
		background: var(--quiet);
	}

	.retrying .key.done {
		background: var(--warn);
	}

	.failed .key.done {
		background: var(--danger);
	}

	.key.arriving {
		background: var(--arriving);
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

	.switch-row {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}

	.times {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 1rem;
	}

	.time {
		display: grid;
		gap: 0.2rem;
	}

	.sub {
		color: var(--muted);
		font-size: 0.8rem;
	}

	.time-row {
		display: flex;
		gap: 0.35rem;
	}

	.time-row input,
	.time-row button {
		height: 2rem;
	}

	/* Radio buttons drawn as one row of segments (as in the filters). */
	.segments {
		display: flex;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--surface);
	}

	/* Holds its hidden radio button: without it, the button would hang from
	   the window and stretch it. */
	.segments label {
		position: relative;
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

	/* Short windows and phones: under the list, as tall as it is; the page
	   scrolls (see Downloads). */
	@media (max-height: 40rem), (max-width: 40rem) {
		.details {
			max-height: none;
			overflow: visible;
		}
	}
</style>
