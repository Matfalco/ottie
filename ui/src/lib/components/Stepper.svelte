<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { parseWhole } from '#lib/settings.js';
	import NumberField from './NumberField.svelte';

	interface Props {
		value: number;
		range: { min: number; max: number };
		/** For screen readers: the setting's name. */
		label: string;
		/** Saves a new value; resolves to false if it couldn't. */
		onChange: (value: number) => Promise<boolean>;
	}

	let { value, range, label, onChange }: Props = $props();

	function step(by: number) {
		const next = Math.min(range.max, Math.max(range.min, value + by));
		if (next !== value) {
			void onChange(next);
		}
	}
</script>

<!-- Fewer, the number, more: one control. -->
<span class="stepper" role="group" aria-label={label}>
	<button aria-label={m.set_fewer()} disabled={value <= range.min} onclick={() => step(-1)}
		>−</button
	>
	<NumberField {value} {label} parse={(text) => parseWhole(text, range)} {onChange} />
	<button aria-label={m.set_more()} disabled={value >= range.max} onclick={() => step(1)}>+</button>
</span>

<style>
	.stepper {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}

	button {
		width: 2.1rem;
		height: 2.1rem;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: var(--hover);
		font-size: 1rem;
	}

	.stepper :global(.field input) {
		width: 3rem;
		border: none;
		border-radius: 999px;
		background: var(--input);
		box-shadow: var(--sunken-shadow);
		text-align: center;
	}
</style>
