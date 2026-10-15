<script lang="ts">
	import icon from '#lib/assets/ottie.svg';
	import { Api, Unauthorized } from '#lib/api.js';
	import { m } from '#lib/paraglide/messages.js';
	import LanguagePicker from './LanguagePicker.svelte';
	import TitleBar from './TitleBar.svelte';

	interface Props {
		onSignIn: (token: string) => void;
	}

	let { onSignIn }: Props = $props();
	let token = $state('');
	let checking = $state(false);
	let problem: string | null = $state(null);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		const candidate = token.trim();
		checking = true;
		problem = null;
		try {
			// Any request tells whether the daemon accepts the token.
			await new Api(candidate).listJobs();
			onSignIn(candidate);
		} catch (error) {
			problem = error instanceof Unauthorized ? m.login_wrong_token() : m.login_unreachable();
		} finally {
			checking = false;
		}
	}
</script>

<TitleBar />

<main>
	<img src={icon} alt="" width="72" height="72" />
	<h1>Ottie</h1>
	<p>{m.login_intro()}</p>
	<form onsubmit={submit}>
		<input
			bind:value={token}
			type="password"
			autocomplete="off"
			placeholder={m.login_token()}
			aria-label={m.login_token()}
			required
		/>
		<button class="primary" disabled={checking || !token.trim()}>{m.login_sign_in()}</button>
	</form>
	{#if problem}
		<p class="problem" role="alert">{problem}</p>
	{/if}
	<p class="hint">{m.login_hint()}</p>
	<LanguagePicker />
</main>

<style>
	main {
		max-width: 26rem;
		margin: 12vh auto 0;
		padding: 0 1.5rem;
		text-align: center;
	}

	img {
		border-radius: var(--radius-l);
	}

	h1 {
		margin: 0.5rem 0 0.25rem;
	}

	form {
		display: flex;
		gap: 0.5rem;
		margin-top: 1.25rem;
	}

	input {
		flex: 1;
		min-width: 0;
	}

	form button {
		white-space: nowrap;
	}

	.problem {
		color: var(--danger);
	}

	.hint {
		margin-top: 2rem;
		color: var(--muted);
		font-size: 0.9em;
	}
</style>
