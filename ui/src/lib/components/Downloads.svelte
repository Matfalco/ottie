<script lang="ts">
	import { onMount } from 'svelte';
	import icon from '#lib/assets/ottie.svg';
	import { currentSpeed } from '#lib/jobs.js';
	import { m } from '#lib/paraglide/messages.js';
	import { Session } from '#lib/session.svelte.js';
	import AddBar from './AddBar.svelte';
	import JobRow from './JobRow.svelte';
	import LanguagePicker from './LanguagePicker.svelte';

	interface Props {
		token: string;
		/** The daemon refused the token. */
		onUnauthorized: () => void;
	}

	let { token, onUnauthorized }: Props = $props();
	// The session lives as long as this page; the token can't change under it.
	// svelte-ignore state_referenced_locally
	const session = new Session(token, onUnauthorized);

	onMount(() => {
		session.start();
		return () => session.stop();
	});
</script>

<header>
	<img src={icon} alt="" width="36" height="36" />
	<h1>Ottie</h1>
	<span class="connection {session.connection}" role="status">
		{#if session.connection === 'live'}
			{m.connection_live()}
		{:else if session.connection === 'connecting'}
			{m.connection_connecting()}
		{:else}
			{m.connection_offline()}
		{/if}
	</span>
	<LanguagePicker />
</header>

<main>
	<AddBar onAdd={(url, destDir) => session.add(url, destDir)} />

	{#if session.error}
		<div class="error" role="alert">
			<span>{session.error}</span>
			<button onclick={() => (session.error = null)} aria-label={m.dismiss()}>✕</button>
		</div>
	{/if}

	{#if session.list.length === 0 && session.connection === 'live'}
		<p class="empty">{m.empty()}</p>
	{:else}
		<ul>
			{#each session.list as job (job.id)}
				<JobRow
					{job}
					speed={currentSpeed(session.rates[job.id], session.now)}
					now={session.now}
					onPause={() => session.pause(job.id)}
					onResume={() => session.resume(job.id)}
					onCancel={() => session.cancel(job.id)}
				/>
			{/each}
		</ul>
	{/if}
</main>

<style>
	header {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		max-width: 56rem;
		margin: 0 auto;
		padding: 1rem 1.5rem 0.5rem;
	}

	header img {
		border-radius: 9px;
	}

	h1 {
		margin: 0;
		font-size: 1.35rem;
	}

	.connection {
		margin-left: auto;
		color: var(--muted);
		font-size: 0.85em;
	}

	.connection::before {
		content: '●';
		margin-right: 0.35em;
	}

	.connection.live::before {
		color: var(--progress);
	}

	.connection.offline {
		color: var(--danger);
	}

	main {
		display: grid;
		gap: 1rem;
		max-width: 56rem;
		margin: 0 auto;
		padding: 0.5rem 1.5rem 2rem;
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
</style>
