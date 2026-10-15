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

	// The language only changes with a reload, so the labels are fixed.
	const labels = LIMIT_PRESETS.map((preset) => formatLimit(preset));
	/** In mono characters, so every column fits the longest ("20 MB/δευτ."). */
	const widest = Math.max(...labels.map((label) => label.length));
</script>

<!-- Common limits, one click each; the one in force is lit. -->
<div class="presets" role="group" aria-label={m.limit_presets()} style:--chars={widest}>
	{#each LIMIT_PRESETS as preset, i (preset)}
		<button
			type="button"
			aria-pressed={limit === preset}
			{disabled}
			onclick={() => void onChange(preset)}>{labels[i]}</button
		>
	{/each}
</div>

<style>
	/* Four columns, fewer where the unit is long ("kB/δευτ."): each at least
	   the longest label (1ch: one mono character) plus its border and a little
	   more (a label may use some of its padding, as "256 kB/s" always has). */
	.presets {
		display: grid;
		grid-template-columns: repeat(
			auto-fill,
			minmax(max(calc(var(--chars) * 1ch + 4px), calc((100% - 0.9rem) / 4)), 1fr)
		);
		gap: 0.3rem;
		font-family: var(--mono);
		font-size: 0.72rem;
	}

	button {
		height: 1.7rem;
		padding: 0 0.25rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--surface);
		color: var(--text);
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
