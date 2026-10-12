<script lang="ts">
	import { onMount } from 'svelte';
	import { countByFilter, filterName, matches, showsNewJobs, type Filter } from '#lib/filters.js';
	import { currentSpeed } from '#lib/jobs.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Session } from '#lib/session.svelte.js';
	import { isFinished } from '#lib/status.js';
	import AddBar from './AddBar.svelte';
	import JobRow from './JobRow.svelte';
	import Sidebar from './Sidebar.svelte';

	interface Props {
		token: string;
		/** The daemon refused the token. */
		onUnauthorized: () => void;
	}

	let { token, onUnauthorized }: Props = $props();
	// The session lives as long as this page; the token can't change under it.
	// svelte-ignore state_referenced_locally
	const session = new Session(token, onUnauthorized);

	let filter: Filter = $state('all');
	let counts = $derived(countByFilter(session.list));
	let shown = $derived(session.list.filter((job) => matches(job, filter)));
	let finished = $derived(shown.filter(isFinished));

	onMount(() => {
		session.start();
		return () => session.stop();
	});

	async function add(url: string, destDir?: string): Promise<boolean> {
		const added = await session.add(url, destDir);
		// Otherwise the new download would be hidden.
		if (added && !showsNewJobs(filter)) {
			filter = 'all';
		}
		return added;
	}
</script>

<div class="page">
	<Sidebar {filter} {counts} connection={session.connection} onSelect={(f) => (filter = f)} />

	<main>
		<AddBar onAdd={add} />

		{#if session.error}
			<div class="error" role="alert">
				<span>{session.error}</span>
				<button onclick={() => (session.error = null)} aria-label={m.dismiss()}>✕</button>
			</div>
		{/if}

		<div class="heading">
			<h2>{filterName(filter)}</h2>
			{#if finished.length > 0}
				<button onclick={() => session.removeAll(finished.map((job) => job.id))}>
					{m.clear_finished()}
				</button>
			{/if}
		</div>

		{#if session.list.length === 0 && session.connection === 'live'}
			<p class="empty">{m.empty()}</p>
		{:else if shown.length === 0 && session.connection === 'live'}
			<p class="empty">{m.empty_filter()}</p>
		{:else}
			<ul>
				{#each shown as job (job.id)}
					<JobRow
						{job}
						speed={currentSpeed(session.rates[job.id], session.now)}
						now={session.now}
						onPause={() => session.pause(job.id)}
						onResume={() => session.resume(job.id)}
						onCancel={() => session.cancel(job.id)}
						onRemove={() => session.remove(job.id)}
					/>
				{/each}
			</ul>
		{/if}
	</main>
</div>

<style>
	.page {
		display: grid;
		/* minmax(0, …): wide content (a long name) scrolls or shrinks rather
		   than widening the page. */
		grid-template-columns: 14rem minmax(0, 1fr);
		min-height: 100vh;
	}

	main {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		align-content: start;
		gap: 1rem;
		width: 100%;
		max-width: 60rem;
		padding: 1.25rem 1.5rem 2rem;
	}

	.heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		min-height: 2.2rem;
	}

	h2 {
		margin: 0;
		font-size: 1.05rem;
	}

	.error {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.6rem 0.9rem;
		border-radius: 8px;
		background: var(--danger-bg);
		color: var(--danger);
	}

	.error button {
		padding: 0 0.4rem;
		border: none;
		background: none;
		color: inherit;
	}

	ul {
		display: grid;
		gap: 0.6rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.empty {
		margin-top: 3rem;
		text-align: center;
		color: var(--muted);
	}

	@media (max-width: 40rem) {
		.page {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: auto 1fr;
		}

		main {
			padding: 1rem;
		}
	}
</style>
