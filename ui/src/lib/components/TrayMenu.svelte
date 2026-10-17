<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import { onMount } from 'svelte';
	import { pauseSteps, resumeTargets } from '#lib/bulk.js';
	import { canDo, FINISH_CHOICES, finishName, nextFinish, soonText } from '#lib/finish.js';
	import { LiveFinishing } from '#lib/finish.svelte.js';
	import { formatCount } from '#lib/format.js';
	import { unfold } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Session } from '#lib/session.svelte.js';
	import { loadToken } from '#lib/token.js';
	import { trayRunning, trayStatus } from '#lib/tray.js';
	import {
		cancelFinish,
		desktop as desktopSetting,
		followDesktop,
		hideMenu,
		menuReady,
		onThisComputer,
		quitApp,
		setFinishOnce,
		setWatchClipboard,
		showMain,
		type Desktop,
		type Finish
	} from '#lib/window.js';
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

	// What happens when everything has finished, this time only, or Cancel
	// during a countdown (docs/decisions/0040).
	const live = new LiveFinishing();
	const finishing = $derived(live.current);
	const countdown = $derived(finishing?.running ?? null);
	/** The choices are listed under their item. */
	let choosing = $state(false);
	/** The desktop app's settings, for watching the clipboard (docs/decisions/0043). */
	let desktop: Desktop | null = $state(null);

	let menu: HTMLElement;

	/** On another computer's Ottie (docs/decisions/0053), only its
	 * downloads: what happens when everything has finished and the
	 * clipboard are this computer's. */
	const local = onThisComputer();
	/** A read-only device's token only looks (docs/decisions/0052). */
	const readOnly = $derived(session?.readOnly ?? false);

	onMount(() => {
		session?.start();
		if (!local) {
			return () => session?.stop();
		}
		live.start();
		const unfollow = followDesktop((next) => (desktop = next));
		desktopSetting().then(
			(next) => (desktop = next),
			() => {}
		);
		return () => {
			session?.stop();
			live.stop();
			unfollow();
		};
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

	function chooseFinish(choice: Finish) {
		choosing = false;
		return live.change(() => setFinishOnce(choice));
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
			<div class="running">
				<RunningList
					running={running.shown}
					rates={session.rates}
					now={session.now}
					slots={session.slotNames}
				/>
				{#if running.more > 0}
					<p class="more">{m.tray_more({ count: formatCount(running.more) })}</p>
				{/if}
			</div>
		{/if}
		<button
			class="item"
			role="menuitem"
			disabled={connection !== 'live' ||
				readOnly ||
				steps.waiting.length + steps.running.length === 0}
			onclick={pick(pauseAll)}>{m.pause_all()}</button
		>
		<button
			class="item"
			role="menuitem"
			disabled={connection !== 'live' || readOnly || resumable.length === 0}
			onclick={pick(resumeAll)}>{m.resume_all()}</button
		>
		{#if finishing}
			{#if countdown}
				<button class="item cancel" role="menuitem" onclick={pick(cancelFinish)}>
					{m.finish_cancel()}
					<span class="soon">{soonText(countdown.action, live.left ?? 0)}</span>
				</button>
			{/if}
			<button
				class="item choose"
				role="menuitem"
				aria-haspopup="menu"
				aria-expanded={choosing}
				onclick={() => (choosing = !choosing)}
			>
				<span>{m.tray_finish({ choice: finishName(nextFinish(finishing)) })}</span>
				<ChevronDown class="chevron" size={14} aria-hidden="true" />
			</button>
			{#if choosing}
				<div class="choices" role="menu" aria-label={m.finish_title()} in:unfold out:unfold>
					{#each FINISH_CHOICES as choice (choice)}
						<button
							class="item"
							role="menuitemradio"
							aria-checked={choice === nextFinish(finishing)}
							class:chosen={choice === nextFinish(finishing)}
							disabled={!canDo(finishing, choice)}
							onclick={pick(() => chooseFinish(choice))}>{finishName(choice)}</button
						>
					{/each}
				</div>
			{/if}
		{/if}
		{#if desktop}
			{@const watching = desktop.watchClipboard}
			<button
				class="item check"
				role="menuitemcheckbox"
				aria-checked={watching}
				onclick={pick(() => setWatchClipboard(!watching))}
			>
				<span>{m.set_watch_clipboard()}</span>
				{#if watching}
					<Check size={14} aria-hidden="true" />
				{/if}
			</button>
		{/if}
		{#if connection === 'live' && limit !== null && !readOnly}
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
		text-align: start;
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

	.choose,
	.check {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.choose span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.choices {
		display: grid;
		padding-inline-start: 0.75rem;
	}

	.chosen {
		color: var(--accent);
		font-weight: 500;
	}

	.cancel {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		color: var(--danger);
		font-weight: 500;
	}

	.soon {
		color: var(--muted);
		font-weight: 400;
		font-size: 0.8rem;
		font-variant-numeric: tabular-nums;
	}
</style>
