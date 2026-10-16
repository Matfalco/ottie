<script lang="ts">
	import { untrack } from 'svelte';
	import { useApp } from '#lib/context.js';
	import AddBar from './AddBar.svelte';
	import BatchDialog from './BatchDialog.svelte';

	const { session, add, adding } = useApp();
	/** The text the list of several links starts from, while it's open. */
	let several: string | null = $state(null);
	let bar: AddBar;

	// A link pasted elsewhere on the page, or "/" (docs/decisions/0046).
	$effect(() => {
		if (adding.pending) {
			untrack(() => {
				const asked = adding.take();
				if (asked) {
					bar.take(asked.link);
				}
			});
		}
	});
</script>

<!-- The add bar on a card of its own, at the top of the Dashboard and the
     Downloads page, and the list for adding several links at once. -->
<div class="card">
	<AddBar
		bind:this={bar}
		categories={session.categories}
		categoryList={session.categoryList}
		streamMedia={session.settings?.streamMedia ?? true}
		onMatch={(url, folder) => session.match(url, folder)}
		onAdd={add}
		onSeveral={(text) => (several = text)}
	/>
</div>

{#if several !== null}
	<BatchDialog text={several} onClose={() => (several = null)} />
{/if}

<style>
	.card {
		padding: 0.6rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	/* Rounded (docs/decisions/0044): a little more room around the pill. */
	:global([data-style='rounded']) .card {
		padding: 0.5rem;
	}
</style>
