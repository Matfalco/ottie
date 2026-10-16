<script lang="ts">
	import type { Component } from 'svelte';
	import type { Job } from '@ottie/api-types/Job';
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import Clock from '@lucide/svelte/icons/clock';
	import Download from '@lucide/svelte/icons/download';
	import Pause from '@lucide/svelte/icons/pause';
	import Play from '@lucide/svelte/icons/play';
	import { resolve } from '$app/paths';
	import { useApp } from '#lib/context.js';
	import { filtersQuery, groupFilters, groupName, groupOf, type Group } from '#lib/filters.js';
	import {
		displayName,
		formatCount,
		formatDuration,
		formatPercent,
		formatSpeed
	} from '#lib/format.js';
	import { currentSpeed } from '#lib/jobs.js';
	import { m } from '#lib/paraglide/messages.js';
	import { canReorder, waitingInOrder } from '#lib/queue.js';
	import { SPANS, loadSpan, saveSpan, type Span } from '#lib/speed.js';
	import { canPause, canResume, describe, fraction } from '#lib/status.js';
	import AddCard from './AddCard.svelte';
	import JobSummary from './JobSummary.svelte';
	import SpeedChart from './SpeedChart.svelte';

	const { session, run } = useApp();
	const downloads = resolve('/downloads');
	/** How many downloads each card lists; "See all" shows the rest. */
	const LISTED = 5;
	const link = (group: Group) => downloads + filtersQuery(groupFilters(group));
	/** Each card's icon, as on the Downloads page's groups. */
	const ICONS: Record<Group, Component> = {
		running: Download,
		waiting: Clock,
		attention: CircleAlert,
		finished: Check
	};

	let span: Span = $state(loadSpan());
	let running = $derived(session.list.filter((job) => job.state === 'downloading'));
	let waiting = $derived(waitingInOrder(session.list));
	let attention = $derived(
		session.list.filter((job) => job.state === 'failed' || job.state === 'retrying')
	);
	let latest = $derived(session.list.filter((job) => job.state === 'completed'));
	let speed = $derived(session.history.at(-1) ?? 0);
	let peak = $derived(Math.max(0, ...session.history.slice(-span)));

	function inGroup(group: Group): number {
		return session.list.filter((job) => groupOf(job) === group).length;
	}

	function choose(chosen: Span) {
		span = chosen;
		saveSpan(chosen);
	}
</script>

<div class="dashboard">
	<AddCard />

	<!-- The speed across the whole width, how far back it looks chosen here. -->
	<section class="card speed" aria-labelledby="dash-speed">
		<header>
			<h2 id="dash-speed">{m.dash_speed()}</h2>
			<div class="spans" role="radiogroup" aria-label={m.dash_span_label()}>
				{#each SPANS as option (option)}
					<label class:chosen={span === option}>
						<input
							type="radio"
							name="span"
							checked={span === option}
							onchange={() => choose(option)}
						/>
						{formatDuration(option)}
					</label>
				{/each}
			</div>
			<span class="numbers">
				{m.dash_now_peak({ now: formatSpeed(speed), peak: formatSpeed(peak) })}
			</span>
		</header>
		<SpeedChart history={session.history} {span} />
	</section>

	<!-- Then the Downloads page's four groups, two by two, each with its
	     first downloads. -->
	<div class="groups">
		<section class="card" aria-labelledby="dash-running">
			{@render head('running', running.length > 0 ? formatSpeed(speed) : null)}
			{#if running.length > 0}
				<ul>
					{#each running.slice(0, LISTED) as job (job.id)}
						{@render runningRow(job)}
					{/each}
				</ul>
			{:else}
				<p class="none">{m.dash_running_none()}</p>
			{/if}
		</section>

		<section class="card" aria-labelledby="dash-waiting">
			{@render head('waiting', null)}
			{#if waiting.length > 0}
				<ul>
					{#each waiting.slice(0, LISTED) as job, index (job.id)}
						{@render waitingRow(job, index + 1)}
					{/each}
				</ul>
			{:else}
				<p class="none">{m.dash_waiting_none()}</p>
			{/if}
		</section>

		<section class="card" aria-labelledby="dash-attention">
			{@render head('attention', null)}
			{#if attention.length > 0}
				<ul>
					{#each attention.slice(0, LISTED) as job (job.id)}
						<JobSummary {job} now={session.now} onRun={(action) => run(action, [job])} />
					{/each}
				</ul>
			{:else}
				<p class="none">{m.dash_attention_none()}</p>
			{/if}
		</section>

		<section class="card" aria-labelledby="dash-finished">
			{@render head('finished', null)}
			{#if latest.length > 0}
				<ul>
					{#each latest.slice(0, LISTED) as job (job.id)}
						<JobSummary {job} now={session.now} />
					{/each}
				</ul>
			{:else}
				<p class="none">{m.dash_finished_none()}</p>
			{/if}
		</section>
	</div>
</div>

<!-- A card's title: the group's icon and name, how many, a word on them, and
     the way to all of them. -->
{#snippet head(group: Group, note: string | null)}
	{@const Icon = ICONS[group]}
	{@const count = inGroup(group)}
	<header>
		<span class="icon {group}" aria-hidden="true"><Icon size={17} /></span>
		<h2 id="dash-{group}">{groupName(group)}</h2>
		{#if count > 0}
			<span class="count {group}">{formatCount(count)}</span>
		{/if}
		{#if note}
			<span class="note">{note}</span>
		{/if}
		{#if count > 0}
			<a href={link(group)}>{m.dash_see_all()}</a>
		{/if}
	</header>
{/snippet}

<!-- A running download: the row fills up as it arrives. -->
{#snippet runningRow(job: Job)}
	{@const name = displayName(job)}
	{@const bytesPerSec = currentSpeed(session.rates[job.id], session.now)}
	{@const done = fraction(job)}
	<li class="row running">
		<span class="fill" class:unknown={done === null} style:width={`${(done ?? 0) * 100}%`}></span>
		<span class="lead percent">{done === null ? '' : formatPercent(done)}</span>
		<span class="text">
			<span class="name" title={job.url} dir="auto">{name}</span>
			<span class="line">
				{describe(job, bytesPerSec, session.now, undefined, session.slotNames)}
			</span>
		</span>
		{#if canPause(job)}
			<button
				class="round"
				onclick={() => run('pause', [job])}
				aria-label={m.pause_label({ name })}
				title={m.pause()}
			>
				<Pause size={15} aria-hidden="true" />
			</button>
		{/if}
	</li>
{/snippet}

<!-- A waiting download: its place in the queue, and the buttons to start it
     or move it. -->
{#snippet waitingRow(job: Job, place: number)}
	{@const name = displayName(job)}
	<li class="row">
		<span class="lead place">{formatCount(place)}</span>
		<span class="text">
			<span class="name" title={job.url} dir="auto">{name}</span>
			<span class="line">
				{describe(job, 0, session.now, undefined, session.slotNames)}
			</span>
		</span>
		{#if canResume(job)}
			<button
				class="round"
				onclick={() => run('resume', [job])}
				aria-label={m.resume_label({ name })}
				title={m.resume()}
			>
				<Play size={15} aria-hidden="true" />
			</button>
		{/if}
		<button
			class="round"
			disabled={!canReorder(session.list, [job], 'up')}
			onclick={() => session.reorder([job.id], 'up')}
			aria-label={m.queue_up_label({ name })}
			title={m.queue_up()}
		>
			<ChevronUp size={16} aria-hidden="true" />
		</button>
		<button
			class="round"
			disabled={!canReorder(session.list, [job], 'down')}
			onclick={() => session.reorder([job.id], 'down')}
			aria-label={m.queue_down_label({ name })}
			title={m.queue_down()}
		>
			<ChevronDown size={16} aria-hidden="true" />
		</button>
	</li>
{/snippet}

<style>
	/* Uses the whole width of the window; cards wrap as it narrows. */
	.dashboard {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 0.25rem 1rem 1rem;
		overflow: auto;
	}

	.card {
		display: flex;
		flex-direction: column;
		min-width: 0;
		padding: 1rem 1.15rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	/* Two by two; one under the other when narrow. */
	.groups {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.75rem;
	}

	@container page (max-width: 44rem) {
		.groups {
			grid-template-columns: minmax(0, 1fr);
		}
	}

	header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem 0.65rem;
		margin-bottom: 0.75rem;
	}

	h2 {
		margin: 0;
		font-size: 1.05rem;
		font-weight: 600;
	}

	/* Room above the graph for its top label. */
	.speed header {
		margin-bottom: 1.6rem;
	}

	.numbers {
		margin-inline-start: auto;
		color: var(--muted);
		font-size: 0.85rem;
	}

	.spans label {
		position: relative;
		border-radius: var(--radius-pill);
		color: var(--muted);
		font-size: 0.85rem;
		cursor: pointer;
		white-space: nowrap;
		padding: 0.2rem 0.8rem;
	}

	.spans label:hover {
		color: var(--text);
	}

	.spans label:focus-within {
		outline: 2px solid var(--focus);
		outline-offset: -2px;
	}

	.spans input {
		position: absolute;
		opacity: 0;
		pointer-events: none;
	}

	/* A group's icon, tinted with its colour. */
	.icon {
		display: grid;
		flex: none;
		place-items: center;
		width: 2.1rem;
		height: 2.1rem;
		border-radius: var(--radius-m);
		--hue: var(--quiet);
		background: color-mix(in srgb, var(--hue) 16%, transparent);
		color: var(--hue);
	}

	.running {
		--hue: var(--progress);
	}

	.waiting {
		--hue: var(--muted);
	}

	.attention {
		--hue: var(--danger);
	}

	.finished {
		--hue: var(--ok);
	}

	.count {
		min-width: 1.4rem;
		padding: 0 0.45rem;
		border-radius: 999px;
		background: color-mix(in srgb, var(--hue) 16%, transparent);
		color: var(--hue);
		font-family: var(--mono);
		font-size: 0.8rem;
		line-height: 1.4rem;
		text-align: center;
	}

	.note {
		color: var(--faint);
		font-size: 0.85rem;
	}

	ul {
		display: grid;
		gap: 0.4rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.none {
		margin: 0.25rem 0;
		color: var(--muted);
	}

	/* One download: what leads it (percent or place), its name and what's
	   going on, then its buttons. */
	.row {
		position: relative;
		display: flex;
		align-items: center;
		gap: 0.65rem;
		min-height: 3.1rem;
		padding: 0.35rem 0.4rem 0.35rem 0.5rem;
		overflow: hidden;
		border-radius: var(--radius-m);
		background: var(--well);
	}

	/* A running one fills up as it arrives, its edge at how far it got. */
	.fill {
		position: absolute;
		inset-block: 0;
		inset-inline-start: 0;
		border-inline-end: 3px solid var(--progress);
		background: color-mix(in srgb, var(--progress) 20%, transparent);
		transition: width 0.25s linear;
	}

	.fill.unknown {
		display: none;
	}

	.lead,
	.text,
	.row button {
		position: relative;
	}

	.lead {
		flex: none;
		width: 2.6rem;
		text-align: center;
	}

	.percent {
		color: var(--progress);
		font-family: var(--mono);
		font-size: 1rem;
		font-weight: 500;
	}

	.place {
		display: grid;
		place-items: center;
		height: 2.3rem;
		border-radius: var(--radius-s);
		background: var(--hover);
		color: var(--muted);
		font-family: var(--mono);
	}

	.text {
		display: grid;
		flex: 1;
		min-width: 0;
	}

	.name,
	.line {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.name {
		font-weight: 500;
	}

	.line {
		color: var(--muted);
		font-size: 0.85em;
	}

	.round {
		display: grid;
		flex: none;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-color: transparent;
		border-radius: 50%;
		background: none;
		color: var(--muted);
	}

	.round:hover:not(:disabled) {
		background: var(--hover);
		color: var(--text);
	}

	/* How far back the graph looks: a segmented choice. Each holds its radio
	   button, which needs a positioned parent. */
	.spans {
		display: flex;
		border-radius: var(--radius-pill);
		padding: 0.2rem;
		border: none;
		background: var(--sunken);
		box-shadow: var(--sunken-shadow);
	}

	.spans label.chosen {
		color: var(--text);
		font-weight: 600;
		background: var(--raised);
		box-shadow: var(--raised-shadow);
	}

	header a {
		margin-inline-start: auto;
		font-size: 0.85rem;
		font-weight: 500;
		text-decoration: none;
		padding: 0.2rem 0.75rem;
		border-radius: 999px;
		background: var(--hover);
		color: var(--text);
	}

	header a:hover {
		background: var(--selected);
		text-decoration: none;
	}

	@media (max-width: 40rem) {
		.dashboard {
			padding: 0.25rem 0.5rem 0.75rem;
			gap: 0.5rem;
		}

		.groups {
			grid-template-columns: minmax(0, 1fr);
			gap: 0.5rem;
		}
	}
</style>
