<script lang="ts">
	import icon from '#lib/assets/ottie.svg';
	import { Api, Unauthorized } from '#lib/api.js';

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
			problem =
				error instanceof Unauthorized
					? "That token isn't right."
					: "Can't reach Ottie. Is `ottie serve` running?";
		} finally {
			checking = false;
		}
	}
</script>

<main>
	<img src={icon} alt="" width="72" height="72" />
	<h1>Ottie</h1>
	<p>Paste the token that <code>ottie token</code> prints.</p>
	<form onsubmit={submit}>
		<input
			bind:value={token}
			type="password"
			autocomplete="off"
			placeholder="Token"
			aria-label="Token"
			required
		/>
		<button class="primary" disabled={checking || !token.trim()}>Sign in</button>
	</form>
	{#if problem}
		<p class="problem" role="alert">{problem}</p>
	{/if}
	<p class="hint">Or run <code>ottie open</code> to open this page signed in.</p>
</main>

<style>
	main {
		max-width: 26rem;
		margin: 12vh auto 0;
		padding: 0 1.5rem;
		text-align: center;
	}

	img {
		border-radius: 16px;
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
