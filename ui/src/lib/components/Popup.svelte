<script lang="ts">
	import type { Settings } from '@ottie/api-types/Settings';
	import Pause from '@lucide/svelte/icons/pause';
	import Play from '@lucide/svelte/icons/play';
	import { onMount, untrack } from 'svelte';
	import { appearance, setAppearance } from '#lib/appearance.svelte.js';
	import { pauseSteps, resumeTargets } from '#lib/bulk.js';
	import { displayName, formatCount } from '#lib/format.js';
	import { currentSpeed } from '#lib/jobs.js';
	import { chosenLanguage, switchLanguage } from '#lib/languages.js';
	import { m } from '#lib/paraglide/messages.js';
	import { isLocale, locales } from '#lib/paraglide/runtime.js';
	import { popupRows, type ToExtension } from '#lib/popup.js';
	import { Session } from '#lib/session.svelte.js';
	import { canPause, canResume, describe, fraction } from '#lib/status.js';
	import { syncSteps } from '#lib/sync.js';
	import { signInLink } from '#lib/token.js';
	import { trayStatus } from '#lib/tray.js';
	import FileIcon from './FileIcon.svelte';

	interface Props {
		token: string;
		/** Tells the extension's popup around this page. */
		tell: (message: ToExtension) => void;
	}

	let { token, tell }: Props = $props();

	// The browser extension's popup (docs/decisions/0055): this page in a
	// frame, signed in with the token the extension handed over. Outside
	// the shell, like the tray menu.
	// svelte-ignore state_referenced_locally
	const session = new Session(token, () => tell({ type: 'ottie-popup-refused' }));

	const jobs = $derived(session.list);
	const connection = $derived(session.connection);
	const speed = $derived(session.history.at(-1) ?? 0);
	const status = $derived(trayStatus(jobs, speed, connection));
	const shown = $derived(popupRows(jobs));
	const steps = $derived(pauseSteps(jobs));
	const resumable = $derived(resumeTargets(jobs));
	const readOnly = $derived(session.readOnly);

	let box: HTMLElement;

	onMount(() => {
		session.start();
		return () => session.stop();
	});

	// The extension sizes its frame to what's drawn here.
	onMount(() => {
		const observer = new ResizeObserver(() =>
			tell({ type: 'ottie-popup-size', height: Math.ceil(box.getBoundingClientRect().height) })
		);
		observer.observe(box);
		return () => observer.disconnect();
	});

	// Ottie's shared look and language, as the main page takes them on (the
	// shell does there); sharing this device's look is the main page's job.
	$effect(() => {
		const settings = session.settings;
		if (settings && appearance.synced) {
			untrack(() => follow(settings));
		}
	});

	function follow(settings: Settings) {
		const steps = syncSteps(appearance, chosenLanguage(), settings, locales);
		if (steps.look) {
			setAppearance(steps.look);
		}
		if (steps.systemLanguage) {
			switchLanguage(null);
		} else if (steps.language && isLocale(steps.language)) {
			switchLanguage(steps.language);
		}
	}

	async function pauseAll() {
		await session.run('pause', steps.waiting);
		await session.run('pause', steps.running);
	}
</script>

<div class="popup" bind:this={box}>
	<p class="status" role="status">{status}</p>
	{#if connection === 'live'}
		{#if shown.rows.length > 0}
			<ul>
				{#each shown.rows as job (job.id)}
					{@const name = displayName(job)}
					{@const rate = currentSpeed(session.rates[job.id], session.now)}
					{@const done = fraction(job)}
					<li>
						<FileIcon {name} size={18} />
						<span class="text">
							<span class="name" title={name} dir="auto">{name}</span>
							<span class="bar" class:unknown={done === null} class:still={rate === 0}>
								<span class="fill" style:width={`${(done ?? 0) * 100}%`}></span>
							</span>
							<span class="line">
								{describe(job, rate, session.now, undefined, session.slotNames)}
							</span>
						</span>
						{#if !readOnly && canPause(job)}
							<button
								class="round"
								onclick={() => void session.run('pause', [job.id])}
								aria-label={m.pause_label({ name })}
								title={m.pause_label({ name })}
							>
								<Pause size={14} aria-hidden="true" />
							</button>
						{:else if !readOnly && canResume(job)}
							<button
								class="round"
								onclick={() => void session.run('resume', [job.id])}
								aria-label={m.resume_label({ name })}
								title={m.resume_label({ name })}
							>
								<Play size={14} aria-hidden="true" />
							</button>
						{/if}
					</li>
				{/each}
			</ul>
			{#if shown.more > 0}
				<p class="more">{m.popup_more({ count: formatCount(shown.more) })}</p>
			{/if}
		{:else}
			<p class="empty">{m.popup_empty()}</p>
		{/if}
	{/if}
	<div class="actions">
		<!-- Signed in as this browser, as its sign-in link would. -->
		<button class="primary" onclick={() => window.open(signInLink(location.origin, token))}>
			{m.tray_open()}
		</button>
		{#if !readOnly}
			<button
				disabled={connection !== 'live' || steps.waiting.length + steps.running.length === 0}
				onclick={() => void pauseAll()}>{m.pause_all()}</button
			>
			<button
				disabled={connection !== 'live' || resumable.length === 0}
				onclick={() => void session.run('resume', resumable)}>{m.resume_all()}</button
			>
		{/if}
	</div>
</div>

<style>
	:global(html),
	:global(body) {
		background: var(--surface);
	}

	.popup {
		display: grid;
		gap: 0.6rem;
		padding: 0.75rem;
		color: var(--text);
		font-size: 0.875rem;
	}

	.status,
	.more,
	.empty {
		margin: 0;
		color: var(--muted);
		font-size: 0.8rem;
	}

	.empty {
		padding: 0.75rem 0;
		text-align: center;
	}

	ul {
		display: grid;
		gap: 0.35rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.5rem 0.6rem;
		border-radius: var(--radius-m);
		background: var(--sunken);
		color: var(--muted);
	}

	.text {
		display: grid;
		flex: 1;
		gap: 0.25rem;
		min-width: 0;
	}

	.name {
		overflow: hidden;
		color: var(--text);
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.bar {
		height: 5px;
		overflow: hidden;
		background: var(--track);
		border-radius: var(--radius-pill);
	}

	.fill {
		display: block;
		height: 100%;
		background: var(--progress);
		transition: width 0.25s linear;
	}

	.still .fill {
		background: var(--quiet);
	}

	/* Size unknown: a moving stripe instead of a percentage. */
	.unknown .fill {
		width: 30% !important;
		animation: slide 1.2s ease-in-out infinite alternate;
	}

	@keyframes slide {
		from {
			margin-inline-start: 0;
		}

		to {
			margin-inline-start: 70%;
		}
	}

	.line {
		overflow: hidden;
		color: var(--faint);
		font-size: 0.8rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.round {
		display: grid;
		flex: none;
		place-items: center;
		width: 1.9rem;
		height: 1.9rem;
		padding: 0;
		border-radius: var(--radius-pill);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}

	.actions > * {
		flex: 1 1 auto;
	}
</style>
