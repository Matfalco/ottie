<script lang="ts">
	import { untrack } from 'svelte';
	import type { Category } from '@ottie/api-types/Category';
	import type { Job } from '@ottie/api-types/Job';
	import type { JobChange } from '@ottie/api-types/JobChange';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Copy from '@lucide/svelte/icons/copy';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import Folder from '@lucide/svelte/icons/folder';
	import ListVideo from '@lucide/svelte/icons/list-video';
	import Play from '@lucide/svelte/icons/play';
	import type { Action } from '#lib/bulk.js';
	import { categoryName } from '#lib/categories.js';
	import {
		PRIORITIES,
		asksAboutFile,
		canStream,
		facts,
		isPlayable,
		primaryAction,
		priorityName
	} from '#lib/details.js';
	import { explainProblem } from '#lib/explain.js';
	import { connections, strip } from '#lib/filemap.js';
	import { statusName, statusOf } from '#lib/filters.js';
	import { displayName, formatBytes, formatPercent, formatSpeed } from '#lib/format.js';
	import { resizes, shuffle, unfold } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { savePlaylist } from '#lib/playlist.js';
	import { canSchedule, fromField, timeChange, toField, type TimeKey } from '#lib/schedule.js';
	import { canOpen } from '#lib/opening.js';
	import { amount, fraction, isFinished } from '#lib/status.js';
	import { playsInApp } from '#lib/window.js';
	import FileBar from './FileBar.svelte';
	import FileIcon from './FileIcon.svelte';
	import Hint from './Hint.svelte';
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
		/** Opens its file, and its folder: null where they can't open (a browser). */
		onOpen: (() => void) | null;
		onShowFolder: (() => void) | null;
		/** A read-only device: it sees, plays and copies links, nothing more. */
		readOnly?: boolean;
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
		getLink,
		onOpen,
		onShowFolder,
		readOnly = false
	}: Props = $props();
	const { session } = useApp();
	let panel: HTMLElement | undefined = $state();
	let priorities: HTMLDivElement | undefined = $state();
	// Folding slides rather than jumps, the chosen priority's highlight to
	// the next (docs/decisions/0048).
	resizes(
		() => panel,
		() => open
	);
	shuffle(
		() => priorities,
		() => job.priority
	);

	let name = $derived(displayName(job));
	let status = $derived(statusOf(job));
	let primary = $derived(primaryAction(job));
	/** It stopped because its file is already there: rename or replace? */
	let asks = $derived(asksAboutFile(job));
	let done = $derived(fraction(job));
	/** Finished or cancelled: its own settings no longer matter, so only
	 * where it is stays (its few facts go with it, under the bar). */
	let over = $derived(isFinished(job));
	let known = $derived(facts(job, speed, now, maxRetries, undefined, session.slotNames));
	/** Beside the name: how much, the whole size once it's done. */
	let size = $derived(
		job.state === 'completed' ? formatBytes(job.total ?? job.downloaded) : amount(job)
	);
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
	/** The daemon gave no play link: its buttons go. Until it answers they
	 * stay in place (inert), so the buttons beside them don't jump back and
	 * forth while the link for another download arrives. */
	let refused = $state(false);
	/** In the desktop app, Play opens the player chosen in Settings. */
	const inApp = playsInApp();
	let copied = $state(false);
	/** The clipboard refused: the link is shown to copy by hand. */
	let showLink = $state(false);

	$effect(() => {
		const wanted = playable ? id : null;
		link = null;
		refused = false;
		copied = false;
		showLink = false;
		if (wanted === null) {
			return;
		}
		let current = true;
		void untrack(getLink).then((url) => {
			if (current) {
				link = url;
				refused = url === null;
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

	/** Answers the question about its file, then starts it again. */
	async function choose(existing: 'rename' | 'replace') {
		if (await onChange({ existing })) {
			onRun('resume');
		}
	}

	function moved(event: Event & { currentTarget: HTMLSelectElement }) {
		const id = Number(event.currentTarget.value);
		if (id !== job.category) {
			onMove(id);
		}
	}
</script>

<!-- The selected download (docs/decisions/0044): a header bar, then two
     halves (its facts and own settings | the bar that maps the file), then a
     strip for where it goes and when. Explanations sit behind an (i). -->
<section class="details {job.state}" aria-label={m.det_label()} bind:this={panel}>
	<div class="top">
		<span class="kind"><FileIcon {name} size={18} /></span>
		<div class="title">
			<h2 title={name} dir="auto">{name}</h2>
			<span class="pill {status}">{statusName(status)}</span>
			<span class="size">{size}</span>
			{#if !open && job.state === 'downloading' && speed > 0}
				<span class="size">{formatSpeed(speed)}</span>
			{/if}
		</div>
		<div class="buttons">
			{#if readOnly}
				<!-- Nothing to start, stop or answer. -->
			{:else if asks}
				<button class="primary" title={m.det_rename_hint()} onclick={() => choose('rename')}>
					{m.existing_rename()}
				</button>
				<button title={m.det_replace_hint()} onclick={() => choose('replace')}>
					{m.existing_replace()}
				</button>
			{:else if primary}
				<button class="primary" onclick={() => onRun(primary.action)}>{primary.label}</button>
			{:else if onOpen && canOpen(job)}
				<button class="primary" onclick={onOpen}>
					<ExternalLink size={13} aria-hidden="true" />
					{m.det_open()}
				</button>
			{/if}
			{#if playable && !refused}
				<!-- A real link: the browser plays it in a new tab (the desktop
				     app hands it to the player chosen in Settings). -->
				<a
					class="button"
					href={link ?? undefined}
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
			{#if onShowFolder}
				<button onclick={onShowFolder}>
					<Folder size={13} aria-hidden="true" />
					{m.det_show_folder()}
				</button>
			{/if}
			{#if !readOnly}
				<button class="danger" onclick={() => onRun('remove')}>{m.remove()}</button>
			{/if}
		</div>
	</div>
	<!-- In the top corner at the end of the line, wherever the buttons wrap to. -->
	<button
		class="toggle"
		aria-expanded={open}
		aria-label={open ? m.det_hide() : m.det_show()}
		title={open ? m.det_hide() : m.det_show()}
		onclick={onToggle}
	>
		<ChevronDown class="chevron rises" size={16} aria-hidden="true" />
	</button>

	{#if showLink && link}
		<input
			class="link"
			readonly
			value={link}
			dir="auto"
			aria-label={m.det_link_label()}
			in:unfold
			out:unfold
		/>
	{/if}

	{#if !open}
		<!-- Folded: the header and a thin bar, so the list gets the room. -->
		<div class="thin">
			<FileBar
				{job}
				size="row"
				role="img"
				aria-label={m.det_map_label({ percent: formatPercent(done ?? 0) })}
			/>
		</div>
	{:else}
		{#if problem}
			<p class="problem" in:unfold out:unfold>{problem}</p>
		{/if}
		<div class="body">
			<div class="column">
				<div class="url" title={job.url} dir="auto">{job.url}</div>
				{#if !over}
					<dl class="facts" style:--count={known.length}>
						{#each known as fact (fact.label)}
							<div>
								<dt>{fact.label}</dt>
								<dd>{fact.value}</dd>
							</div>
						{/each}
					</dl>
					<div class="tune">
						<div class="inline" role="radiogroup" aria-label={m.det_priority()}>
							<span class="label" aria-hidden="true">{m.det_priority()}</span>
							<Hint text={m.det_priority_hint()} />
							<div class="segments" bind:this={priorities}>
								{#each PRIORITIES as priority (priority)}
									<label class:chosen={job.priority === priority}>
										{#if job.priority === priority}
											<span class="highlight" data-flip="highlight" data-flip-size></span>
										{/if}
										<input
											type="radio"
											name="priority"
											checked={job.priority === priority}
											disabled={readOnly}
											onchange={() => onChange({ priority })}
										/>
										{priorityName(priority)}
									</label>
								{/each}
							</div>
						</div>
						<div class="inline">
							<span class="label">{m.det_speed_limit()}</span>
							<Hint text={m.det_limit_hint()} />
							<LimitControl
								limit={job.speedLimit}
								label={m.det_limit_switch()}
								memory="download"
								disabled={readOnly}
								onChange={(speedLimit) => onChange({ speedLimit })}
							/>
						</div>
					</div>
				{/if}
			</div>

			<div class="column map">
				<div class="amount">
					<span class="label">{m.col_progress()}</span>
					<span class="percent">{done === null ? '' : formatPercent(done)}</span>
				</div>
				<FileBar
					{job}
					size="detail"
					role="img"
					aria-label={m.det_map_label({ percent: formatPercent(done ?? 0) })}
				/>
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
			</div>
		</div>

		<div class="strip">
			{#if over}
				{#each known as fact (fact.label)}
					<div class="field">
						<span class="label">{fact.label}</span>
						<span class="value">{fact.value}</span>
					</div>
				{/each}
			{/if}
			<label class="field">
				<span class="label">{m.det_category()}</span>
				<select value={job.category} disabled={readOnly} onchange={moved}>
					{#each choices as category (category.id)}
						<option value={category.id}>{categoryName(category)}</option>
					{/each}
				</select>
			</label>
			<div class="field grow">
				<span class="label">{m.det_folder()}</span>
				<span class="folder" title={job.destDir} dir="auto">{job.destDir}</span>
			</div>
			{#if canStream(job)}
				<div class="field">
					<span class="name">
						<span class="label">{m.det_streaming()}</span>
						<Hint text={m.det_streaming_hint()} />
					</span>
					<span class="control">
						<Switch
							on={job.streaming}
							label={m.det_streaming()}
							disabled={readOnly}
							onChange={(streaming) => onChange({ streaming })}
						/>
					</span>
				</div>
			{/if}
			{#if canSchedule(job)}
				<!-- One group, so Stop at wraps right under Start at. -->
				<div class="times" role="group" aria-label={m.det_schedule()}>
					{@render timeField(m.det_start_at(), 'startAt', job.startAt, m.det_schedule_hint())}
					{@render timeField(m.det_stop_at(), 'stopAt', job.stopAt, null)}
				</div>
			{/if}
		</div>
	{/if}
</section>

{#snippet timeField(label: string, key: TimeKey, value: number | null, hint: string | null)}
	<div class="field">
		<span class="name">
			<span class="label">{label}</span>
			{#if hint}
				<Hint text={hint} />
			{/if}
		</span>
		<span class="time-row">
			<input
				type="datetime-local"
				aria-label={label}
				value={toField(value)}
				disabled={readOnly}
				onblur={(event) => commit(event.currentTarget, key, value)}
				onkeydown={(event) => event.key === 'Enter' && event.currentTarget.blur()}
			/>
			{#if value !== null && !readOnly}
				<button type="button" onclick={() => onChange(timeChange(key, null))}>
					{m.det_clear_time()}
				</button>
			{/if}
		</span>
	</div>
{/snippet}

<style>
	/* A read-only device: the priorities as they are, dimmed. */
	.segments:has(input:disabled) {
		opacity: 0.6;
	}

	/* Under the list, on the same card; at most half of it, and less when
	   the list needs its room (see JobTable), down to its header. It
	   scrolls by itself beyond that. */
	.details {
		position: relative;
		flex: 0 1 auto;
		min-height: 3.6rem;
		max-height: 50%;
		overflow: auto;
		border-top: 1px solid var(--border);
		background: var(--well);
		margin: 0.25rem 0.5rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		box-shadow: 0 -8px 24px rgb(0 0 0 / 16%);
	}

	/* Room at the end for the arrow. */
	.top {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
		padding-block: 0.55rem;
		padding-inline: 0.75rem 3.4rem;
	}

	/* The file's kind, as in the Dashboard's rows. */
	.kind {
		display: grid;
		flex: none;
		place-items: center;
		width: 2.4rem;
		height: 2.4rem;
		background: var(--hover);
		color: var(--muted);
		border-radius: var(--radius-m);
	}

	.downloading .kind {
		background: var(--selected);
		color: var(--text);
	}

	.failed .kind {
		background: var(--danger-bg);
		color: var(--danger);
	}

	.retrying .kind {
		background: var(--warn-bg);
		color: var(--warn);
	}

	.completed .kind {
		background: var(--ok-bg);
		color: var(--ok);
	}

	.title {
		display: flex;
		flex: 1 1 18rem;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.2rem 0.6rem;
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

	.size {
		flex: none;
		color: var(--muted);
		font-family: var(--mono);
		font-size: 0.8rem;
		white-space: nowrap;
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
		border-radius: var(--radius-pill);
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

	.thin {
		padding: 0 0.9rem 0.7rem;
	}

	/* Side by side when both fit, the facts' half wider (its priority and
	   limit share a line); one under the other on narrow windows. */
	.body {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem 1.5rem;
		padding: 0.1rem 0.9rem 0.75rem;
	}

	.column {
		display: grid;
		flex: 3 1 17rem;
		grid-template-columns: minmax(0, 1fr);
		align-content: start;
		gap: 0.6rem;
		min-width: 0;
	}

	/* The bar halfway down its side, level with the facts. */
	.column.map {
		flex-grow: 2;
		align-content: center;
	}

	.facts {
		display: grid;
		grid-template-columns: repeat(var(--count), minmax(0, 1fr));
		gap: 0.5rem;
		margin: 0;
	}

	dt,
	.label {
		color: var(--faint);
		font-size: 0.8rem;
		white-space: nowrap;
	}

	dd {
		margin: 0.1rem 0 0;
		overflow: hidden;
		font-family: var(--mono);
		font-size: 0.9rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Priority and the speed limit side by side, each with its name. */
	.tune {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 1.25rem;
	}

	.inline {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem;
		min-width: 0;
	}

	.amount {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}

	.percent {
		color: var(--muted);
		font-family: var(--mono);
		font-size: 0.85rem;
	}

	.map-foot {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 0.3rem 0.9rem;
		color: var(--faint);
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
		background: var(--track);
		border-radius: 999px;
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

	.field {
		display: grid;
		align-content: end;
		gap: 0.3rem;
		min-width: 0;
	}

	.field.grow {
		flex: 1 1 12rem;
	}

	.name {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		min-height: 1.25rem;
	}

	.control,
	.value {
		display: flex;
		align-items: center;
		height: 2rem;
	}

	.value {
		font-family: var(--mono);
		font-size: 0.9rem;
		white-space: nowrap;
	}

	select {
		min-width: 9rem;
		height: 2rem;
		padding-block: 0;
	}

	.folder {
		display: flex;
		align-items: center;
		height: 2rem;
		padding: 0 0.6rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-pill);
		background: var(--input);
	}

	/* Start and stop: side by side, or one under the other when they don't
	   fit, never split across the strip. */
	.times {
		display: flex;
		flex: 0 1 auto;
		flex-wrap: wrap;
		gap: 0.6rem 1rem;
		min-width: 0;
	}

	.time-row {
		display: flex;
		gap: 0.35rem;
	}

	.time-row input,
	.time-row button {
		height: 2rem;
	}

	.segments label:not(.chosen):hover {
		background: var(--hover);
	}

	/* Only when reached from the keyboard: a click shows the choice by the
	   highlight alone. */
	.segments label:has(input:focus-visible) {
		outline: 2px solid var(--focus);
		outline-offset: -2px;
	}

	.segments input {
		position: absolute;
		opacity: 0;
		pointer-events: none;
	}

	@media (max-width: 40rem) {
		.facts {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	/* Short windows and phones: under the list, as tall as it is; the page
	   scrolls (see Downloads). */
	@media (max-height: 40rem), (max-width: 40rem) {
		.details {
			flex: none;
			max-height: none;
			overflow: visible;
		}
	}

	.toggle {
		position: absolute;
		top: 0.75rem;
		inset-inline-end: 0.9rem;
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-color: transparent;
		color: var(--muted);
		border-radius: 50%;
		background: var(--hover);
	}

	/* The link in a sunken pill; the folder too. */
	.url,
	.folder {
		overflow: hidden;
		color: var(--muted);
		font-family: var(--mono);
		font-size: 0.8rem;
		text-overflow: ellipsis;
		white-space: nowrap;
		padding: 0.45rem 0.9rem;
		border: none;
		border-radius: 999px;
		background: var(--input);
		box-shadow: var(--sunken-shadow);
	}

	.folder {
		padding-block: 0;
	}

	/* The facts as tiles, sunk like the link above them. */
	.facts > div {
		padding: 0.45rem 0.75rem;
		border-radius: var(--radius-m);
		background: var(--input);
		box-shadow: var(--sunken-shadow);
	}

	/* Where it goes and when: each control with its name above it. */
	.strip {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: 0.6rem 1rem;
		margin: 0 0.5rem 0.5rem;
		padding: 0.6rem 0.75rem 0.7rem;
		border-top: none;
		border-radius: var(--radius-l);
		background: var(--bg);
	}

	/* Radio buttons drawn as one row of segments (as in the filters). */
	.segments {
		/* Its own layer: the chosen one's highlight slides under every
		   item's text (each item isolated, it would cover the earlier ones). */
		isolation: isolate;
		display: flex;
		overflow: hidden;
		gap: 0.15rem;
		padding: 0.2rem;
		border: none;
		border-radius: 999px;
		background: var(--sunken);
		box-shadow: var(--sunken-shadow);
	}

	/* Holds its hidden radio button: without it, the button would hang from
	   the window and stretch it. */
	.segments label {
		position: relative;
		display: grid;
		flex: 1;
		place-items: center;
		padding: 0 0.6rem;
		color: var(--muted);
		cursor: pointer;
		min-height: 1.75rem;
		border-radius: 999px;
	}

	.segments label + label {
		border-inline-start: none;
	}

	.segments label.chosen {
		color: var(--text);
		font-weight: 500;
	}

	.problem {
		margin: 0 0.9rem 0.6rem;
		padding: 0.5rem 0.75rem;
		background: var(--danger-bg);
		color: var(--danger);
		font-size: 0.85rem;
		border-radius: var(--radius-pill);
		padding-inline: 1rem;
	}
</style>
