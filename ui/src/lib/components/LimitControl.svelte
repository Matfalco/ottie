<script lang="ts">
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import {
		SPEED_UNITS,
		fieldToLimit,
		limitToField,
		loadLastLimit,
		saveLastLimit,
		unitLabel,
		type LimitMemory
	} from '#lib/settings.js';
	import Switch from './Switch.svelte';

	interface Props {
		/** Bytes per second; 0 = no limit. */
		limit: number;
		disabled?: boolean;
		/** The switch's name. Default: the total limit's. */
		label?: string;
		/** Which last limit to remember (default: the total one's). */
		memory?: LimitMemory;
		/** Saves a new limit (0 = none); resolves to false if it couldn't. */
		onChange: (bytesPerSec: number) => Promise<boolean>;
	}

	let {
		limit,
		disabled = false,
		label = m.limit_total(),
		memory = 'total',
		onChange
	}: Props = $props();
	/** Shown while the limit is off, and set again when it's switched on. */
	let last = $state(untrack(() => loadLastLimit(memory)));
	const on = $derived(limit > 0);
	/** The number and its unit, from the saved limit: after a change, the
	 * unit is the largest the new limit has one of. */
	const field = $derived(limitToField(on ? limit : last));

	$effect(() => {
		if (limit > 0) {
			last = limit;
			saveLastLimit(
				limit,
				untrack(() => memory)
			);
		}
	});

	/** Saves the number typed (on Enter or on leaving the field); what can't
	 * be saved puts the current limit back. */
	async function typed(event: Event & { currentTarget: HTMLInputElement }) {
		const input = event.currentTarget;
		const bytes = fieldToLimit(input.value, field.unit);
		if (bytes === null || bytes === limit || !(await onChange(bytes))) {
			input.value = field.value;
		}
	}

	/** Another unit: the same number in it ("40" MB/s → "40" kB/s). */
	async function unitChosen(event: Event & { currentTarget: HTMLSelectElement }) {
		const select = event.currentTarget;
		const unit = SPEED_UNITS.find((known) => known === select.value);
		const bytes = unit ? fieldToLimit(field.value, unit) : null;
		if (bytes === null || !(await onChange(bytes))) {
			select.value = field.unit;
		}
	}
</script>

<!-- A speed limit: on or off, and how much. -->
<span class="limit">
	<Switch {on} {label} {disabled} onChange={(next) => void onChange(next ? last : 0)} />
	<input
		value={field.value}
		inputmode="decimal"
		aria-label={m.limit_value()}
		disabled={disabled || !on}
		onchange={typed}
	/>
	<select
		value={field.unit}
		aria-label={m.limit_unit()}
		disabled={disabled || !on}
		onchange={unitChosen}
	>
		{#each SPEED_UNITS as unit (unit)}
			<option value={unit}>{unitLabel(unit)}</option>
		{/each}
	</select>
</span>

<style>
	.limit {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.limit > :global(button) {
		margin-inline-end: 0.25rem;
	}

	input {
		width: 4.5rem;
		height: 2.1rem;
		text-align: end;
	}

	select {
		height: 2.1rem;
	}
</style>
