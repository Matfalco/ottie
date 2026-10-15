<script lang="ts">
	import { toField } from '#lib/settings.js';

	interface Props {
		value: number;
		/** For screen readers: the setting's name. */
		label: string;
		/** Shown after the field ("MB/s"); "" keeps the room for one, so
		 * fields line up. */
		unit?: string;
		/** What was typed, as a value; null if it can't be one. */
		parse: (text: string) => number | null;
		/** A value as the field shows it. */
		show?: (value: number) => string;
		disabled?: boolean;
		/** Saves a new value; resolves to false if it couldn't. */
		onChange: (value: number) => Promise<boolean>;
	}

	let {
		value,
		label,
		unit,
		parse,
		show = (value) => toField(value),
		disabled = false,
		onChange
	}: Props = $props();

	/** Saves on Enter or on leaving the field; anything that can't be saved
	 * puts the current value back. */
	async function commit(event: Event & { currentTarget: HTMLInputElement }) {
		const input = event.currentTarget;
		const typed = parse(input.value);
		if (typed === null || typed === value || !(await onChange(typed))) {
			input.value = show(value);
		}
	}
</script>

<span class="field">
	<input value={show(value)} inputmode="decimal" aria-label={label} {disabled} onchange={commit} />
	{#if unit !== undefined}
		<span class="unit">{unit}</span>
	{/if}
</span>

<style>
	.field {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	input {
		width: 4.5rem;
		height: 2.1rem;
		text-align: end;
	}

	.unit {
		min-width: 2.5rem;
		color: var(--muted);
	}
</style>
