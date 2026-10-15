<script lang="ts">
	import { resolve } from '$app/paths';
	import { useApp } from '#lib/context.js';
	import {
		GROUPS,
		filtersQuery,
		groupFilters,
		groupName,
		groupOf,
		type Group
	} from '#lib/filters.js';
	import { formatCount, formatSpeed } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { upNext } from '#lib/queue.js';
	import AddCard from './AddCard.svelte';
	import JobSummary from './JobSummary.svelte';
	import RunningList from './RunningList.svelte';
	import SpeedChart from './SpeedChart.svelte';

	const { session, run } = useApp();
	const downloads = resolve('/downloads');
	/** How many finished downloads the dashboard lists. */
	const LATEST = 5;
	/** How many waiting downloads "Up next" lists. */
	const NEXT = 3;
	/** The Downloads page's four groups, in the mockup's order. */
	const TILES: Group[] = ['running', 'waiting', 'attention', 'finished'];
	const link = (group: Group) => downloads + filtersQuery(groupFilters(group));

	let counts = $derived(
		Object.fromEntries(
			GROUPS.map((group) => [group, session.list.filter((job) => groupOf(job) === group).length])
		) as Record<Group, number>
	);
	let attention = $derived(
		session.list.filter((job) => job.state === 'failed' || job.state === 'retrying')
	);
	let running = $derived(session.list.filter((job) => job.state === 'downloading'));
	let next = $derived(upNext(session.list).slice(0, NEXT));
	let latest = $derived(session.list.filter((job) => job.state === 'completed').slice(0, LATEST));
	let speed = $derived(session.history.at(-1) ?? 0);
	let peak = $derived(Math.max(0, ...session.history));
</script>

<div class="dashboard">
	<AddCard />

	<div class="tiles">
		{#each TILES as group (group)}
			<a class="card tile {group}" href={link(group)}>
				<span class="label"><span class="dot" aria-hidden="true"></span>{groupName(group)}</span>
				<span class="value">{formatCount(counts[group])}</span>
			</a>
		{/each}
	</div>

	<div class="row">
		<section class="card speed" aria-labelledby="dash-speed">
			<header>
				<h2 id="dash-speed">{m.dash_speed()}</h2>
				<span class="faint">{m.dash_last_minutes()}</span>
				<span class="numbers">
					{m.dash_now_peak({ now: formatSpeed(speed), peak: formatSpeed(peak) })}
				</span>
			</header>
			<SpeedChart history={session.history} />
		</section>

		<section class="card now" aria-labelledby="dash-running">
			<header>
				<h2 id="dash-running">{m.dash_running()}</h2>
				{#if running.length > 0 || next.length > 0}
					<a href={link('running')}>{m.dash_see_all()}</a>
				{/if}
			</header>
			<RunningList {running} {next} rates={session.rates} now={session.now} />
		</section>
	</div>

	<div class="lists">
		<section class="card" aria-labelledby="dash-attention">
			<header>
				<h2 id="dash-attention">{m.dash_attention()}</h2>
				{#if attention.length > 0}
					<span class="badge">{formatCount(attention.length)}</span>
					<a href={link('attention')}>{m.dash_see_all()}</a>
				{/if}
			</header>
			{#if attention.length > 0}
				<ul>
					{#each attention as job (job.id)}
						<JobSummary {job} now={session.now} onRun={(action) => run(action, [job])} />
					{/each}
				</ul>
			{:else}
				<p class="none">{m.dash_attention_none()}</p>
			{/if}
		</section>

		<section class="card" aria-labelledby="dash-finished">
			<header>
				<h2 id="dash-finished">{m.dash_finished()}</h2>
				{#if latest.length > 0}
					<a href={link('finished')}>{m.dash_see_all()}</a>
				{/if}
			</header>
			{#if latest.length > 0}
				<ul>
					{#each latest as job (job.id)}
						<JobSummary {job} now={session.now} />
					{/each}
				</ul>
			{:else}
				<p class="none">{m.dash_finished_none()}</p>
			{/if}
		</section>
	</div>
</div>

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
		min-width: 0;
		padding: 1rem 1.15rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	/* Four in a row, or two by two: never three and one. */
	.tiles {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 0.75rem;
	}

	@container page (max-width: 48rem) {
		.tiles {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	.tile {
		display: grid;
		gap: 0.15rem;
		color: var(--text);
		text-decoration: none;
	}

	.tile:hover {
		border-color: var(--muted);
	}

	.label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--muted);
	}

	.dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: var(--quiet);
	}

	.running .dot {
		background: var(--progress);
	}

	.attention .dot {
		background: var(--danger);
	}

	.attention .value {
		color: var(--danger);
	}

	.finished .dot {
		background: var(--ok);
	}

	.value {
		font-size: 2rem;
		font-weight: 600;
		line-height: 1.15;
	}

	/* The graph takes what "Downloading now" leaves; below about 52rem they stack. */
	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.speed {
		flex: 999 1 30rem;
	}

	.now {
		flex: 1 1 20rem;
	}

	.lists {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(22rem, 1fr));
		gap: 0.75rem;
	}

	header {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.4rem 0.75rem;
		margin-bottom: 0.75rem;
	}

	h2 {
		margin: 0;
		font-size: 1.05rem;
		font-weight: 600;
	}

	.faint {
		color: var(--faint);
		font-size: 0.9em;
	}

	.numbers {
		margin-left: auto;
		color: var(--muted);
		font-size: 0.85rem;
	}

	/* Room above the graph for its top label. */
	.speed header {
		margin-bottom: 1.6rem;
	}

	header a {
		margin-left: auto;
		color: var(--accent);
		font-size: 0.85rem;
		font-weight: 500;
		text-decoration: none;
	}

	header a:hover {
		text-decoration: underline;
	}

	.badge {
		padding: 0 0.5rem;
		border-radius: 999px;
		background: var(--danger-bg);
		color: var(--danger);
		font-size: 0.8rem;
		font-weight: 600;
		line-height: 1.3rem;
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.none {
		margin: 0.25rem 0;
		color: var(--muted);
	}

	@media (max-width: 40rem) {
		.dashboard {
			padding: 0.25rem 0.5rem 0.75rem;
			gap: 0.5rem;
		}

		.tiles {
			grid-template-columns: repeat(2, minmax(0, 1fr));
			gap: 0.5rem;
		}

		.lists {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
