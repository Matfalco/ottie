<script lang="ts">
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import { formatCount } from '#lib/format.js';
	import { resizes } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** How many downloads are selected: none, or several. */
		count: number;
		/** Whether the lower part is shown (shared with JobDetails). */
		open: boolean;
		onToggle: () => void;
	}

	let { count, open, onToggle }: Props = $props();
	let panel: HTMLElement | undefined = $state();
	// Folding slides rather than jumps (docs/decisions/0048).
	resizes(
		() => panel,
		() => open
	);
</script>

<!-- The details panel while it has no one download to show: it keeps its
     place under the list, as JobDetails draws it. -->
<section class="details" aria-label={m.det_label()} bind:this={panel}>
	<div class="top">
		<!-- One line: what's selected, then what to do. -->
		<div class="text">
			<h2>
				{count === 0 ? m.det_none() : m.det_several({ count: formatCount(count) })}
			</h2>
			{#if open}
				<p class="hint">{count === 0 ? m.det_none_hint() : m.det_several_hint()}</p>
			{/if}
		</div>
		<button
			class="toggle"
			aria-expanded={open}
			aria-label={open ? m.det_hide() : m.det_show()}
			title={open ? m.det_hide() : m.det_show()}
			onclick={onToggle}
		>
			{#if open}
				<ChevronDown size={16} aria-hidden="true" />
			{:else}
				<ChevronUp size={16} aria-hidden="true" />
			{/if}
		</button>
	</div>
</section>

<style>
	.top {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		min-height: 2rem;
		padding: 0.45rem 0.75rem 0.45rem 0.9rem;
	}

	.text {
		display: flex;
		flex: 1;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.2rem 0.75rem;
		min-width: 0;
	}

	h2 {
		margin: 0;
		color: var(--muted);
		font-size: 1rem;
		font-weight: 600;
	}

	.hint {
		margin: 0;
		color: var(--faint);
		font-size: 0.85rem;
	}
	/* As JobDetails, so switching between them doesn't jump. */
	.details {
		flex: none;
		border-top: 1px solid var(--border);
		background: var(--well);
		margin: 0.25rem 0.5rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		box-shadow: 0 -8px 24px rgb(0 0 0 / 16%);
	}

	.toggle {
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-color: transparent;
		color: var(--muted);
		border-radius: 50%;
		background: var(--hover);
	}
</style>
