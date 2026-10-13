<script lang="ts">
	import type { Matched } from '@ottie/api-types/Matched';
	import { categoryHue, categoryName, type Categories } from '#lib/categories.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		categories: Categories;
		/** The category a link would get; null if the daemon can't tell. */
		onMatch: (url: string) => Promise<Matched | null>;
	}

	let { categories, onMatch }: Props = $props();
	let url = $state('');
	let matched = $state<Matched | null>(null);
	let typing: ReturnType<typeof setTimeout> | undefined;

	let category = $derived(matched ? categories[matched.category] : undefined);

	$effect(() => () => clearTimeout(typing));

	/** Asks the daemon once the user stops typing for a moment. */
	function typed() {
		clearTimeout(typing);
		const asked = url.trim();
		if (!asked) {
			matched = null;
			return;
		}
		typing = setTimeout(async () => {
			const answer = await onMatch(asked);
			if (url.trim() === asked) {
				matched = answer;
			}
		}, 300);
	}
</script>

<!-- "Where would a link go?": the category and folder a pasted link would
     get, without adding it. -->
<div class="tester">
	<label for="try-link">{m.cat_try()}</label>
	<input
		id="try-link"
		type="url"
		bind:value={url}
		oninput={typed}
		placeholder={m.cat_try_placeholder()}
		spellcheck="false"
	/>
	{#if category}
		<p class="answer" aria-live="polite">
			<span class="dot" style:--hue={categoryHue(category)} aria-hidden="true"></span>
			<strong>{categoryName(category)}</strong>
			{#if matched?.extension}
				<span class="faint">(.{matched.extension})</span>
			{/if}
			<span class="faint" aria-hidden="true">→</span>
			<span class="folder">{category.folder ?? m.cat_no_folder()}</span>
		</p>
	{/if}
</div>

<style>
	.tester {
		display: grid;
		gap: 0.4rem;
		padding: 0.75rem 0.9rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	label {
		color: var(--muted);
		font-size: 0.85rem;
	}

	input {
		min-width: 0;
		font-family: var(--mono);
		font-size: 0.8rem;
	}

	.answer {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem;
		margin: 0;
		font-size: 0.85rem;
	}

	.dot {
		width: 0.55rem;
		height: 0.55rem;
		border-radius: 3px;
		background: var(--hue);
	}

	.faint {
		color: var(--faint);
	}

	.folder {
		min-width: 0;
		overflow: hidden;
		color: var(--muted);
		font-family: var(--mono);
		font-size: 0.8rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
