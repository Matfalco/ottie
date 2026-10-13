<script lang="ts">
	import { resolve } from '$app/paths';
	import { useApp } from '#lib/context.js';
	import { countByFilter, filterName, filterQuery, type Filter } from '#lib/filters.js';
	import { formatCount, formatSpeed } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import JobSummary from './JobSummary.svelte';
	import SpeedChart from './SpeedChart.svelte';

	const { session, run } = useApp();
	const downloads = resolve('/downloads');
	/** How many finished downloads the dashboard lists. */
	const LATEST = 5;
	const TILES: Filter[] = ['active', 'queued', 'stopped', 'completed'];

	let counts = $derived(countByFilter(session.list));
	let attention = $derived(
		session.list.filter((job) => job.state === 'failed' || job.state === 'retrying')
	);
	// The list is newest first, so these are the most recently added.
	let latest = $derived(session.list.filter((job) => job.state === 'completed').slice(0, LATEST));
	let speed = $derived(session.history.at(-1) ?? 0);
</script>

<div class="dashboard">
	<section class="card speed">
		<h2>{m.dash_speed()}</h2>
		<p class="hero">{formatSpeed(speed)}</p>
		<SpeedChart history={session.history} />
	</section>

	<div class="tiles">
		{#each TILES as filter (filter)}
			<a class="card tile {filter}" href={downloads + filterQuery(filter)}>
				<span class="value">{formatCount(counts[filter])}</span>
				<span class="label">{filterName(filter)}</span>
			</a>
		{/each}
	</div>

	<section class="card">
		<header>
			<h2>{m.dash_attention()}</h2>
			{#if attention.length > 0}
				<a href={downloads + filterQuery('stopped')}>{m.dash_see_all()}</a>
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

	<section class="card">
		<header>
			<h2>{m.dash_finished()}</h2>
			{#if latest.length > 0}
				<a href={downloads + filterQuery('completed')}>{m.dash_see_all()}</a>
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

<style>
	.dashboard {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		align-content: start;
		gap: 1rem;
		width: 100%;
		max-width: 72rem;
		padding: 1.25rem;
		overflow: auto;
	}

	.card {
		padding: 1rem 1.1rem;
		border: 1px solid var(--border);
		border-radius: 12px;
		background: var(--surface);
	}

	.speed,
	.tiles {
		grid-column: 1 / -1;
	}

	h2 {
		margin: 0;
		color: var(--muted);
		font-size: 0.85rem;
		font-weight: 600;
	}

	/* The one big number on the page. */
	.hero {
		margin: 0.2rem 0 0.75rem;
		font-size: 3rem;
		font-weight: 600;
		line-height: 1.1;
	}

	.tiles {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 1rem;
	}

	.tile {
		display: grid;
		gap: 0.1rem;
		color: var(--text);
		text-decoration: none;
	}

	.tile:hover {
		border-color: var(--muted);
	}

	.value {
		font-size: 1.75rem;
		font-weight: 600;
	}

	.label {
		color: var(--muted);
		font-size: 0.9em;
	}

	header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		margin-bottom: 0.4rem;
	}

	header a {
		color: var(--accent);
		font-size: 0.85rem;
		text-decoration: none;
	}

	header a:hover {
		text-decoration: underline;
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.none {
		margin: 0.75rem 0 0.25rem;
		color: var(--muted);
	}

	@media (max-width: 56rem) {
		.dashboard {
			grid-template-columns: minmax(0, 1fr);
		}

		.tiles {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	@media (max-width: 40rem) {
		.dashboard {
			padding: 0.75rem;
			gap: 0.75rem;
		}

		.tiles {
			gap: 0.75rem;
		}

		.hero {
			font-size: 2.5rem;
		}
	}
</style>
