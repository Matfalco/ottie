<script lang="ts">
	import { onMount, untrack, type Snippet } from 'svelte';
	import type { AddJob } from '@ottie/api-types/AddJob';
	import type { AddedJobs } from '@ottie/api-types/AddedJobs';
	import type { Job } from '@ottie/api-types/Job';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import type { Settings } from '@ottie/api-types/Settings';
	import { appearance, setAppearance } from '#lib/appearance.svelte.js';
	import { confirmation, targets, type Action } from '#lib/bulk.js';
	import { provideApp } from '#lib/context.js';
	import { parseFilters, showsNewJobs } from '#lib/filters.js';
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, isLocale, locales, setLocale } from '#lib/paraglide/runtime.js';
	import { Session, type Answer } from '#lib/session.svelte.js';
	import { syncSteps } from '#lib/sync.js';
	import ConfirmDialog from './ConfirmDialog.svelte';
	import NavMenu from './NavMenu.svelte';
	import Setup from './Setup.svelte';
	import SpeedChip from './SpeedChip.svelte';
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
	/** The first start: the setup page replaces the app until it's done. */
	let settingUp = $derived(session.settings?.setupDone === false);

	// svelte-ignore state_referenced_locally
	provideApp({ session, token, run, ask, add, addMany });

	// A device that follows the shared look and language takes them on
	// whenever they change, and shares its own while none are shared. (Only
	// the settings and the choice to follow are watched: a change made here is
	// on its way to the daemon and comes back as the shared one.)
	$effect(() => {
		const settings = session.settings;
		if (settings && appearance.synced) {
			untrack(() => void follow(settings));
		}
	});

	async function follow(settings: Settings) {
		const steps = syncSteps(appearance, getLocale(), settings, locales);
		if (steps.look) {
			setAppearance(steps.look);
		}
		if (steps.share) {
			await session.changeSettings(steps.share);
		}
		if (steps.language && isLocale(steps.language)) {
			setLocale(steps.language); // Reloads the page in that language.
		}
	}

	onMount(() => {
		session.start();
		return () => session.stop();
	});

	async function add(job: AddJob): Promise<boolean> {
		const added = await session.add(job);
		if (added) {
			await showNewJobs();
		}
		return added;
	}

	async function addMany(jobs: AddJob[]): Promise<Answer<AddedJobs>> {
		const answer = await session.addMany(jobs);
		if (answer.ok && answer.value.ids.some((id) => id !== null)) {
			await showNewJobs();
		}
		return answer;
	}

	/** On a Downloads view that would hide new downloads, shows everything instead. */
	async function showNewJobs() {
		const downloads = resolve('/downloads');
		const filters = parseFilters(page.url.searchParams);
		if (page.url.pathname === downloads && !showsNewJobs(filters)) {
			await goto(downloads, { replace: true, reset: false });
		}
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

	async function setLimit(bytesPerSec: number): Promise<boolean> {
		const why = await session.changeSettings({ globalLimit: bytesPerSec });
		if (why !== null) {
			session.error = why;
		}
		return why === null;
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
		{#if !settingUp}
			<NavMenu jobs={session.list} />
		{/if}
		<span class="end">
			<SpeedChip
				speed={session.history.at(-1) ?? 0}
				limit={session.settings?.globalLimit ?? null}
				connection={session.connection}
				onLimit={setLimit}
			/>
		</span>
	</TitleBar>

	{#if settingUp}
		<Setup />
	{:else}
		<main>
			{#if session.error}
				<div class="error" role="alert">
					<span>{session.error}</span>
					<button onclick={() => (session.error = null)} aria-label={m.dismiss()}>✕</button>
				</div>
			{/if}
			{@render children()}
		</main>
	{/if}
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
		/* minmax(0, …): wide content scrolls or shrinks rather than widening the page. */
		grid-template-columns: minmax(0, 1fr);
		grid-template-rows: auto minmax(0, 1fr);
		height: 100vh;
		height: 100dvh;
	}

	/* Pushes the speed to the right of the title bar. */
	.end {
		margin-left: auto;
	}

	main {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		/* Pages adapt to the room they get, not to the window:
		   `@container page (…)` in their styles. */
		container: page / inline-size;
	}

	.error {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		margin: 0.75rem 0.75rem 0;
		padding: 0.6rem 0.9rem;
		border-radius: var(--radius-s);
		background: var(--danger-bg);
		color: var(--danger);
	}

	.error button {
		padding: 0 0.4rem;
		border: none;
		background: none;
		color: inherit;
	}

	/* Phones: room for the tab bar along the bottom. */
	@media (max-width: 40rem) {
		main {
			padding-bottom: 3.7rem;
		}
	}
</style>
