<script lang="ts">
	interface Props {
		on: boolean;
		/** What it turns on, for screen readers. */
		label: string;
		disabled?: boolean;
		onChange: (on: boolean) => void;
	}

	let { on, label, disabled = false, onChange }: Props = $props();
</script>

<button
	type="button"
	role="switch"
	aria-checked={on}
	aria-label={label}
	{disabled}
	class:on
	onclick={() => onChange(!on)}
>
	<span class="knob" aria-hidden="true"></span>
</button>

<style>
	button {
		position: relative;
		flex: none;
		width: 2.25rem;
		height: 1.25rem;
		padding: 0;
		border: none;
		border-radius: 999px;
		background: var(--track);
	}

	button.on {
		background: var(--accent);
	}

	.knob {
		position: absolute;
		top: 0.1875rem;
		left: 0.1875rem;
		width: 0.875rem;
		height: 0.875rem;
		border-radius: 50%;
		background: var(--muted);
		transition: left 0.15s;
	}

	.on .knob {
		left: 1.1875rem;
		background: var(--on-accent);
	}
</style>
