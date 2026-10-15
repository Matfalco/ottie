<script lang="ts">
	import type { Category } from '@ottie/api-types/Category';
	import FolderInput from '@lucide/svelte/icons/folder-input';
	import { categoryName } from '#lib/categories.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		categories: Category[];
		disabled: boolean;
		onMove: (category: number) => void;
	}

	let { categories, disabled, onMove }: Props = $props();
	let open = $state(false);

	/** Closes the list once focus leaves the menu. */
	function focusOut(event: FocusEvent & { currentTarget: HTMLElement }) {
		const next = event.relatedTarget;
		if (!(next instanceof Node && event.currentTarget.contains(next))) {
			open = false;
		}
	}

	function choose(id: number) {
		open = false;
		onMove(id);
	}
</script>

<div class="menu" onfocusout={focusOut}>
	<button
		{disabled}
		aria-haspopup="menu"
		aria-expanded={open}
		aria-label={m.move_label()}
		title={m.move_label()}
		onclick={() => (open = !open)}
	>
		<FolderInput size={15} aria-hidden="true" />
		<span>{m.move_to()}</span>
	</button>
	{#if open && !disabled}
		<ul role="menu">
			{#each categories as category (category.id)}
				<li role="none">
					<button
						role="menuitem"
						onclick={() => choose(category.id)}
						onkeydown={(event) => event.key === 'Escape' && (open = false)}
					>
						{categoryName(category)}
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.menu {
		position: relative;
	}

	button {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		height: 2rem;
		padding: 0 0.6rem;
		border-color: transparent;
		background: none;
		white-space: nowrap;
	}

	button:hover:not(:disabled) {
		border-color: transparent;
		background: var(--hover);
	}

	button:disabled {
		opacity: 0.4;
	}

	ul {
		position: absolute;
		top: calc(100% + 0.3rem);
		inset-inline-start: 0;
		z-index: 10;
		min-width: 12rem;
		max-height: 18rem;
		margin: 0;
		padding: 0.3rem;
		overflow: auto;
		list-style: none;
		border: 1px solid var(--border);
		border-radius: var(--radius-m);
		background: var(--surface);
		box-shadow: var(--shadow);
	}

	ul button {
		width: 100%;
		height: 2.1rem;
		border-radius: var(--radius-s);
		text-align: start;
	}

	@container page (max-width: 56rem) {
		.menu > button span {
			display: none;
		}
	}
</style>
