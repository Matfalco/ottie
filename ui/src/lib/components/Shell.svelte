<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import type { Job } from '@ottie/api-types/Job';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { confirmation, targets, type Action } from '#lib/bulk.js';
	import { provideApp } from '#lib/context.js';
	import { parseFilter, showsNewJobs } from '#lib/filters.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Session } from '#lib/session.svelte.js';
	import AddBar from './AddBar.svelte';
	import ConfirmDialog from './ConfirmDialog.svelte';
	import NavMenu from './NavMenu.svelte';
	import TitleBar from './TitleBar.svelte';

	interface Props {
		token: string;
		/** The daemon refused the token. */
		onUnauthorized: () => void;
		/** The page. */
		children: Snippet;
	}

	interface Question {
		message: string;
		confirmLabel: string;
		answer: (yes: boolean) => void;
	}

	let { token, onUnauthorized, children }: Props = $props();
	// One session for every page, so moving between them keeps the live
	// connection. The token can't change under it.
	// svelte-ignore state_referenced_locally
	const session = new Session(token, onUnauthorized);
	let question: Question | null = $state(null);

	provideApp({ session, run, ask });

	onMount(() => {
		session.start();
		return () => session.stop();
	});

	async function add(url: string, destDir?: string): Promise<boolean> {
		const added = await session.add(url, destDir);
		// On a Downloads view that would hide it, show everything instead.
		const downloads = resolve('/downloads');
		const filter = parseFilter(page.url.searchParams.get('status'));
		if (added && page.url.pathname === downloads && !showsNewJobs(filter)) {
			await goto(downloads, { replace: true, reset: false });
		}
		return added;
	}

	async function run(action: Action, jobs: Job[]): Promise<void> {
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
</script>

<div class="app">
	<TitleBar>
		<AddBar onAdd={add} />
	</TitleBar>

	<div class="body">
		<NavMenu connection={session.connection} jobs={session.list} />

		<main>
			{#if session.error}
				<div class="error" role="alert">
					<span>{session.error}</span>
					<button onclick={() => (session.error = null)} aria-label={m.dismiss()}>✕</button>
				</div>
			{/if}
			{@render children()}
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
	/* The app fills the window; each page scrolls inside `main`. */
	.app {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		height: 100vh;
		height: 100dvh;
	}

	.body {
		display: grid;
		/* minmax(0, …): wide content scrolls or shrinks rather than widening the page. */
		grid-template-columns: var(--nav-width) minmax(0, 1fr);
		min-height: 0;
	}

	main {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		background: var(--bg);
		/* Pages adapt to the room left beside the menu, not to the window:
		   `@container page (…)` in their styles. */
		container: page / inline-size;
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

	/* Phones: the menu moves to a bar at the bottom. */
	@media (max-width: 40rem) {
		.body {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: minmax(0, 1fr) auto;
		}

		main {
			grid-row: 1;
		}
	}
</style>
