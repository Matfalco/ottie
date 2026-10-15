<script lang="ts">
	import '@fontsource-variable/outfit';
	import '@fontsource/dm-mono/400.css';
	import '@fontsource/dm-mono/500.css';
	import '../app.css';
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import { page } from '$app/state';
	import { appearance } from '#lib/appearance.svelte.js';
	import icon from '#lib/assets/ottie.svg';
	import Login from '#lib/components/Login.svelte';
	import Shell from '#lib/components/Shell.svelte';
	import { direction, followLanguage } from '#lib/languages.js';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import { themeFor } from '#lib/theme.js';
	import { forgetToken, loadToken, saveToken, tokenFromFragment } from '#lib/token.js';
	import { tellLanguage } from '#lib/window.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
	let token: string | null = $state(null);
	let ready = $state(false);
	/** The desktop app's tray menu: no shell around it (docs/decisions/0034). */
	const inTray = page.route.id === '/tray';

	// For screen readers, spell checking and hyphenation, and which way the
	// page runs: mirrored for a language written right to left. (Changing
	// the language reloads the page, so this runs again.)
	const shown = getLocale();
	document.documentElement.lang = shown;
	document.documentElement.dir = direction(shown);
	// The desktop app's tray speaks it too.
	tellLanguage(shown);
	// A language chosen in another window (or the shared one, taken on
	// there) reloads this one too: the tray's menu stays loaded while hidden.
	onMount(() => followLanguage(shown));

	// The theme, set before anything is drawn, then kept up to date with the
	// user's choice and the system's dark mode.
	const darkSystem = matchMedia('(prefers-color-scheme: dark)');
	let systemDark = $state(darkSystem.matches);
	document.documentElement.dataset.theme = themeFor(appearance, darkSystem.matches);
	$effect(() => {
		document.documentElement.dataset.theme = themeFor(appearance, systemDark);
	});

	onMount(() => {
		const follow = (event: MediaQueryListEvent) => (systemDark = event.matches);
		darkSystem.addEventListener('change', follow);
		return () => darkSystem.removeEventListener('change', follow);
	});

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
	{#if inTray}
		{@render children()}
	{:else if token}
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
