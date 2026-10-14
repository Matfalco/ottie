<script lang="ts">
	import { onMount } from 'svelte';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Session } from '#lib/session.svelte.js';
	import { loadToken } from '#lib/token.js';
	import { pauseSteps, resumeTargets, trayRunning, trayStatus } from '#lib/tray.js';
	import { hideMenu, menuReady, quitApp, showMain } from '#lib/window.js';
	import LimitControl from './LimitControl.svelte';
	import RatePresets from './RatePresets.svelte';
	import RunningList from './RunningList.svelte';

	// Ottie's own tray menu in the desktop app (docs/decisions/0034): the
	// app opens it at the tray icon, sized to what's drawn here. Signed in
	// like the main window (the same page storage); without a token, only
	// opening and quitting.
	const token = loadToken();
	// The main window deals with a refused token.
	const session = token ? new Session(token, () => {}) : null;

	const jobs = $derived(session?.list ?? []);
	const connection = $derived(session?.connection ?? 'offline');
	const speed = $derived(session?.history.at(-1) ?? 0);
	const limit = $derived(session?.settings?.globalLimit ?? null);
	const status = $derived(trayStatus(jobs, speed, connection));
	const running = $derived(trayRunning(jobs));
	const steps = $derived(pauseSteps(jobs));
	const resumable = $derived(resumeTargets(jobs));

	let menu: HTMLElement;

	onMount(() => {
		session?.start();
		return () => session?.stop();
	});

	// The app sizes the window to the menu, now and whenever it changes.
	onMount(() => {
		const observer = new ResizeObserver(() =>
			menuReady(Math.ceil(menu.getBoundingClientRect().height))
		);
		observer.observe(menu);
		return () => observer.disconnect();
	});

	/** Like a menu: picking an item closes it. */
	function pick(action: () => Promise<unknown>) {
		return () => {
			void hideMenu().catch(() => {});
			void action().catch(() => {});
		};
	}

	async function pauseAll() {
		await session?.run('pause', steps.waiting);
		await session?.run('pause', steps.running);
	}

	async function resumeAll() {
		await session?.run('resume', resumable);
	}

	async function setLimit(bytesPerSec: number): Promise<boolean> {
		return (await session?.changeSettings({ globalLimit: bytesPerSec })) === null;
	}

	function keyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			void hideMenu().catch(() => {});
		}
	}
</script>

<svelte:window onkeydown={keyDown} />

<div class="menu" role="menu" aria-label={m.tray_menu()} bind:this={menu}>
	<button class="item strong" role="menuitem" onclick={pick(showMain)}>{m.tray_open()}</button>
	{#if session}
		<hr />
		<p class="status" role="status">{status}</p>
		{#if connection === 'live' && running.shown.length > 0}
			<!-- Like the dashboard's "Downloading now": each with its bar. -->
			<div class="running">
				<RunningList running={running.shown} next={[]} rates={session.rates} now={session.now} />
				{#if running.more > 0}
					<p class="more">{m.tray_more({ count: formatCount(running.more) })}</p>
				{/if}
			</div>
		{/if}
		<button
			class="item"
			role="menuitem"
			disabled={connection !== 'live' || steps.waiting.length + steps.running.length === 0}
			onclick={pick(pauseAll)}>{m.tray_pause_all()}</button
		>
		<button
			class="item"
			role="menuitem"
			disabled={connection !== 'live' || resumable.length === 0}
			onclick={pick(resumeAll)}>{m.tray_resume_all()}</button
		>
		{#if connection === 'live' && limit !== null}
			<hr />
			<div class="limit">
				<p class="title">{m.limit_total()}</p>
				<LimitControl {limit} onChange={setLimit} />
				<RatePresets {limit} onChange={setLimit} />
			</div>
		{/if}
	{/if}
	<hr />
	<button class="item" role="menuitem" onclick={pick(quitApp)}>{m.tray_quit()}</button>
</div>

<style>
	:global(html),
	:global(body) {
		background: var(--surface);
	}

	.menu {
		display: grid;
		padding: 0.35rem;
		border: 1px solid var(--border);
		background: var(--surface);
		color: var(--text);
		font-size: 0.875rem;
		user-select: none;
	}

	.item {
		height: 2rem;
		padding: 0 0.75rem;
		border: none;
		border-radius: var(--radius-s);
		background: none;
		color: var(--text);
		text-align: left;
	}

	.item:hover:not(:disabled),
	.item:focus-visible {
		background: var(--hover);
	}

	.item:disabled {
		color: var(--faint);
	}

	.strong {
		font-weight: 600;
	}

	.status {
		margin: 0;
		padding: 0.35rem 0.75rem;
		color: var(--muted);
		font-size: 0.8rem;
	}

	.running {
		display: grid;
		gap: 0.6rem;
		padding: 0.15rem 0.75rem 0.5rem;
	}

	.more {
		margin: 0;
		color: var(--faint);
		font-size: 0.8rem;
	}

	hr {
		width: 100%;
		margin: 0.3rem 0;
		border: none;
		border-top: 1px solid var(--border-soft);
	}

	.limit {
		display: grid;
		gap: 0.55rem;
		padding: 0.35rem 0.75rem 0.5rem;
	}

	.title {
		margin: 0;
		color: var(--muted);
		font-size: 0.8rem;
	}
</style>
