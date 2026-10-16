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
	/* A sunken track that lights up, and a raised knob with a grip. */
	button {
		position: relative;
		flex: none;
		width: 2.6rem;
		height: 1.5rem;
		padding: 0;
		border: none;
		border-radius: 999px;
		background: var(--sunken);
		box-shadow: var(--sunken-shadow);
		transition: box-shadow 0.15s;
	}

	/* Its colour when on, as a layer that fades in: a gradient can't change
	   colour smoothly itself (docs/decisions/0048). */
	button::before {
		content: '';
		position: absolute;
		inset: 0;
		border-radius: inherit;
		background: linear-gradient(
			180deg,
			color-mix(in srgb, var(--accent) 80%, black),
			var(--accent)
		);
		opacity: 0;
		transition: opacity 0.15s;
	}

	button.on {
		box-shadow:
			inset 0 1px 3px rgb(0 0 0 / 30%),
			0 0 10px color-mix(in srgb, var(--accent) 30%, transparent);
	}

	button.on::before {
		opacity: 1;
	}

	.knob {
		position: absolute;
		top: 0.1875rem;
		inset-inline-start: 0.1875rem;
		width: 1.125rem;
		height: 1.125rem;
		border-radius: 50%;
		background:
			repeating-conic-gradient(var(--surface) 0 25%, var(--well) 0 50%) 0 0 / 4px 4px,
			var(--surface);
		box-shadow:
			0 2px 4px rgb(0 0 0 / 35%),
			inset 0 0 0 1px var(--border),
			inset 0 1px 0 rgb(255 255 255 / 12%);
		transition: inset-inline-start 0.15s;
	}

	.on .knob {
		inset-inline-start: 1.2875rem;
	}
</style>
