<script lang="ts">
	import { onMount } from 'svelte';
	import type { Job } from '@ottie/api-types/Job';
	import { confirmation, targets, type Action } from '#lib/bulk.js';
	import { countByFilter, matches, showsNewJobs, type Filter } from '#lib/filters.js';
	import { m } from '#lib/paraglide/messages.js';
	import { NOTHING, keepShown, selectAll, type Selection } from '#lib/selection.js';
	import { Session } from '#lib/session.svelte.js';
	import { isFinished } from '#lib/status.js';
	import AddBar from './AddBar.svelte';
	import ConfirmDialog from './ConfirmDialog.svelte';
	import JobTable from './JobTable.svelte';
	import Sidebar from './Sidebar.svelte';
	import TitleBar from './TitleBar.svelte';
	import Toolbar from './Toolbar.svelte';

	interface Props {
		token: string;
		/** The daemon refused the token. */
		onUnauthorized: () => void;
	}

	interface Question {
		message: string;
		confirmLabel: string;
		answer: (yes: boolean) => void;
	}

	let { token, onUnauthorized }: Props = $props();
	// The session lives as long as this page; the token can't change under it.
	// svelte-ignore state_referenced_locally
	const session = new Session(token, onUnauthorized);

	let filter: Filter = $state('all');
	// Replaced as a whole on every change, never edited in place.
	let selection: Selection = $state.raw(NOTHING);
	let question: Question | null = $state(null);

	let counts = $derived(countByFilter(session.list));
	let shown = $derived(session.list.filter((job) => matches(job, filter)));
	let order = $derived(shown.map((job) => job.id));
	// Downloads that were removed or left the view drop out of the selection.
	let current = $derived(keepShown(selection, order));
	let selected = $derived(shown.filter((job) => current.ids.has(job.id)));
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

	/** Does `action` to the jobs it applies to, asking first if that would
	 * delete downloaded data. */
	async function run(action: Action, jobs: Job[]) {
		const chosen = targets(jobs, action);
		const message = confirmation(action, chosen);
		if (message !== null) {
			const confirmLabel = action === 'cancel' ? m.cancel_yes() : m.remove_yes();
			if (!(await ask(message, confirmLabel))) {
				return;
			}
		}
		await session.run(
			action,
			chosen.map((job) => job.id)
		);
	}

	function ask(message: string, confirmLabel: string): Promise<boolean> {
		return new Promise((resolve) => {
			question = {
				message,
				confirmLabel,
				answer: (yes) => {
					question = null;
					resolve(yes);
				}
			};
		});
	}

	/** File-manager keys: Ctrl+A, Escape, Delete. Not while typing. */
	function keydown(event: KeyboardEvent) {
		const target = event.target;
		const typing =
			(target instanceof HTMLInputElement && target.type !== 'checkbox') ||
			target instanceof HTMLTextAreaElement ||
			target instanceof HTMLSelectElement;
		if (typing || question) {
			return;
		}
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
			event.preventDefault();
			selection = selectAll(order);
		} else if (event.key === 'Escape') {
			selection = NOTHING;
		} else if (event.key === 'Delete' && selected.length > 0) {
			void run('remove', selected);
		}
	}
</script>

<svelte:window onkeydown={keydown} />

<div class="app">
	<TitleBar>
		<AddBar onAdd={add} />
	</TitleBar>

	<div class="body">
		<Sidebar
			{filter}
			{counts}
			connection={session.connection}
			onSelect={(chosen) => (filter = chosen)}
		/>

		<main>
			{#if session.error}
				<div class="error" role="alert">
					<span>{session.error}</span>
					<button onclick={() => (session.error = null)} aria-label={m.dismiss()}>✕</button>
				</div>
			{/if}

			<Toolbar {selected} {finished} onRun={run} />

			{#if session.list.length === 0 && session.connection === 'live'}
				<p class="empty">{m.empty()}</p>
			{:else if shown.length === 0 && session.connection === 'live'}
				<p class="empty">{m.empty_filter()}</p>
			{:else}
				<JobTable
					jobs={shown}
					selection={current}
					rates={session.rates}
					now={session.now}
					onSelect={(chosen) => (selection = chosen)}
					onRun={run}
				/>
			{/if}
		</main>
	</div>
</div>

{#if question}
	<ConfirmDialog
		message={question.message}
		confirmLabel={question.confirmLabel}
		onAnswer={question.answer}
	/>
{/if}

<style>
	/* The page fills the window; only the list scrolls. */
	.app {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		height: 100vh;
		height: 100dvh;
	}

	.body {
		display: grid;
		/* minmax(0, …): wide content scrolls or shrinks rather than widening the page. */
		grid-template-columns: 13rem minmax(0, 1fr);
		min-height: 0;
	}

	main {
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--bg);
	}

	.error {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		margin: 0.75rem 0.75rem 0;
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

	.empty {
		margin-top: 4rem;
		padding: 0 1rem;
		text-align: center;
		color: var(--muted);
	}

	@media (max-width: 40rem) {
		.body {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: auto minmax(0, 1fr);
		}
	}
</style>
