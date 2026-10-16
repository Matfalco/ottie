<script lang="ts">
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** How many downloads are selected: none, or several. */
		count: number;
		/** Whether the lower part is shown (shared with JobDetails). */
		open: boolean;
		onToggle: () => void;
	}

	let { count, open, onToggle }: Props = $props();
</script>

<!-- The details panel while it has no one download to show: it keeps its
     place under the list, as JobDetails draws it. -->
<section class="details" aria-label={m.det_label()}>
	<div class="top">
		<h2>
			{count === 0 ? m.det_none() : m.det_several({ count: formatCount(count) })}
		</h2>
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
	{#if open}
		<p class="hint">{count === 0 ? m.det_none_hint() : m.det_several_hint()}</p>
	{/if}
</section>

<style>
	/* As JobDetails, so switching between them doesn't jump. */
	.details {
		flex: none;
		border-top: 1px solid var(--border);
		background: var(--well);
	}

	.top {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		min-height: 2rem;
		padding: 0.6rem 0.9rem;
	}

	h2 {
		flex: 1;
		margin: 0;
		color: var(--muted);
		font-size: 1.05rem;
		font-weight: 600;
	}

	.toggle {
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	.hint {
		margin: 0;
		padding: 0 0.9rem 1rem;
		color: var(--faint);
		font-size: 0.9rem;
	}

	/* Rounded (docs/decisions/0044): a raised card of its own under the
	   list, rather than the pane's bottom. */
	:global([data-style='rounded']) .details {
		margin: 0.25rem 0.5rem 0.5rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		box-shadow: 0 -8px 24px rgb(0 0 0 / 16%);
	}

	:global([data-style='rounded']) .toggle {
		border-radius: 50%;
		background: var(--hover);
	}
</style>
