<script lang="ts">
	import { formatLimit } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { LIMIT_PRESETS } from '#lib/settings.js';

	interface Props {
		/** Bytes per second; 0 = no limit. */
		limit: number;
		disabled?: boolean;
		/** Saves a new limit; resolves to false if it couldn't. */
		onChange: (bytesPerSec: number) => Promise<boolean>;
	}

	let { limit, disabled = false, onChange }: Props = $props();
</script>

<!-- Common limits, one click each; the one in force is lit. -->
<div class="presets" role="group" aria-label={m.limit_presets()}>
	{#each LIMIT_PRESETS as preset (preset)}
		<button
			type="button"
			aria-pressed={limit === preset}
			{disabled}
			onclick={() => void onChange(preset)}>{formatLimit(preset)}</button
		>
	{/each}
</div>

<style>
	.presets {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 0.3rem;
	}

	button {
		height: 1.7rem;
		padding: 0 0.25rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--surface);
		color: var(--text);
		font-family: var(--mono);
		font-size: 0.72rem;
		white-space: nowrap;
	}

	button:hover:not(:disabled) {
		background: var(--hover);
	}

	button[aria-pressed='true'] {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--on-accent);
	}
</style>
