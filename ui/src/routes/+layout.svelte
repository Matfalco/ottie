<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import icon from '#lib/assets/ottie.svg';
	import Login from '#lib/components/Login.svelte';
	import Shell from '#lib/components/Shell.svelte';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { forgetToken, loadToken, saveToken, tokenFromFragment } from '#lib/token.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
	let token: string | null = $state(null);
	let ready = $state(false);

	// For screen readers, spell checking and hyphenation. (Changing the
	// language reloads the page, so this runs again.)
	document.documentElement.lang = getLocale();

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

<svelte:head>
	<link rel="icon" href={icon} />
</svelte:head>

{#if ready}
	{#if token}
		<!-- Keyed: a new token means a new session. -->
		{#key token}
			<Shell {token} onUnauthorized={signOut}>
				{@render children()}
			</Shell>
		{/key}
	{:else}
		<Login onSignIn={signIn} />
	{/if}
{/if}
