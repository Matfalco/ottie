<script lang="ts">
	import { useApp } from '#lib/context.js';
	import AddBar from './AddBar.svelte';

	const { session, add } = useApp();
</script>

<!-- The add bar on a card of its own, at the top of the Dashboard and the
     Downloads page. -->
<div class="card">
	<AddBar
		categories={session.categories}
		categoryList={session.categoryList}
		onMatch={(url, folder) => session.match(url, folder)}
		onAdd={add}
	/>
</div>

<style>
	.card {
		padding: 0.6rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
	}
</style>
