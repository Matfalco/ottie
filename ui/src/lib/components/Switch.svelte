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
		padding: 0;
		border: none;
		border-radius: 999px;
		width: 2.6rem;
		height: 1.5rem;
		background: var(--sunken);
		box-shadow: var(--sunken-shadow);
	}

	.knob {
		position: absolute;
		inset-inline-start: 0.1875rem;
		border-radius: 50%;
		transition: inset-inline-start 0.15s;
		top: 0.1875rem;
		width: 1.125rem;
		height: 1.125rem;
		background:
			repeating-conic-gradient(var(--surface) 0 25%, var(--well) 0 50%) 0 0 / 4px 4px,
			var(--surface);
		box-shadow:
			0 2px 4px rgb(0 0 0 / 35%),
			inset 0 0 0 1px var(--border),
			inset 0 1px 0 rgb(255 255 255 / 12%);
	}

	.on .knob {
		inset-inline-start: 1.2875rem;
	}

	button.on {
		background: linear-gradient(
			180deg,
			color-mix(in srgb, var(--accent) 80%, black),
			var(--accent)
		);
		box-shadow:
			inset 0 1px 3px rgb(0 0 0 / 30%),
			0 0 10px color-mix(in srgb, var(--accent) 30%, transparent);
	}
</style>
