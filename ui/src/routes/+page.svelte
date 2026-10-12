<script lang="ts">
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import Downloads from '#lib/components/Downloads.svelte';
	import Login from '#lib/components/Login.svelte';
	import { forgetToken, loadToken, saveToken, tokenFromFragment } from '#lib/token.js';

	let token: string | null = $state(null);
	let ready = $state(false);

	onMount(() => {
		const fromLink = tokenFromFragment(location.hash);
		if (fromLink) {
			saveToken(fromLink);
			// Take it out of the address bar and the history entry.
			replaceState(location.pathname + location.search, {});
		}
		token = fromLink ?? loadToken();
		ready = true;
	});

	function signIn(newToken: string) {
		saveToken(newToken);
		token = newToken;
	}

	function signOut() {
		forgetToken();
		token = null;
	}
</script>

{#if ready}
	{#if token}
		<Downloads {token} onUnauthorized={signOut} />
	{:else}
		<Login onSignIn={signIn} />
	{/if}
{/if}
