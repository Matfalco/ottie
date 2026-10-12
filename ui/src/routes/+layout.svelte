<script lang="ts">
	import '../app.css';
	import icon from '#lib/assets/ottie.svg';
	import { getLocale } from '#lib/paraglide/runtime.js';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();

	// For screen readers, spell checking and hyphenation. (Changing the
	// language reloads the page, so this runs again.)
	document.documentElement.lang = getLocale();
</script>

<svelte:head>
	<link rel="icon" href={icon} />
</svelte:head>

{@render children()}
