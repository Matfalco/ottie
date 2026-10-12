<script lang="ts">
	import '../app.css';
	import icon from '#lib/assets/ottie.svg';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<svelte:head>
	<link rel="icon" href={icon} />
</svelte:head>

{@render children()}
