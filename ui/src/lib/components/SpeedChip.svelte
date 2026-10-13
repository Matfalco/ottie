<script lang="ts">
	import Gauge from '@lucide/svelte/icons/gauge';
	import { formatSpeed } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { Connection } from '#lib/session.svelte.js';
	import LimitControl from './LimitControl.svelte';

	interface Props {
		/** All downloads together, in bytes per second. */
		speed: number;
		/** The total limit in bytes per second (0 = none); null until known. */
		limit: number | null;
		connection: Connection;
		/** Saves a new total limit; resolves to false if it couldn't. */
		onLimit: (bytesPerSec: number) => Promise<boolean>;
	}

	let { speed, limit, connection, onLimit }: Props = $props();
	let open = $state(false);
	let chip: HTMLButtonElement | undefined = $state();

	const label = $derived(
		limit
			? m.chip_speed_limited({ speed: formatSpeed(speed), limit: formatSpeed(limit) })
			: m.chip_speed({ speed: formatSpeed(speed) })
	);

	/** Closes the popup once focus leaves the chip and the popup. */
	function focusOut(event: FocusEvent & { currentTarget: HTMLElement }) {
		const next = event.relatedTarget;
		if (!(next instanceof Node && event.currentTarget.contains(next))) {
			open = false;
		}
	}

	function keyDown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			open = false;
			chip?.focus();
		}
	}
</script>

<!-- The total speed in the title bar, and the total limit if there is one;
     clicking it opens the limit. While the daemon can't be reached, why
     instead. -->
{#if connection === 'live' && limit !== null}
	<div class="holder" onfocusout={focusOut} onkeydown={keyDown} role="presentation">
		<button
			bind:this={chip}
			class="chip"
			aria-label={label}
			title={label}
			aria-expanded={open}
			aria-haspopup="dialog"
			onclick={() => (open = !open)}
		>
			<Gauge size={14} aria-hidden="true" />
			<span class="speed">{formatSpeed(speed)}</span>
			{#if limit > 0}
				<span class="limit">/ {formatSpeed(limit)}</span>
			{/if}
		</button>
		{#if open}
			<!-- Focusable, so a click inside doesn't count as leaving it. -->
			<div class="popup" role="dialog" aria-label={m.limit_total()} tabindex="-1">
				<p class="title">{m.limit_total()}</p>
				<LimitControl {limit} onChange={onLimit} />
				<p class="hint">{m.limit_chip_hint()}</p>
			</div>
		{/if}
	</div>
{:else if connection === 'live'}
	<span class="chip" title={m.dash_speed()}>
		<Gauge size={14} aria-hidden="true" />
		<span class="speed">{formatSpeed(speed)}</span>
	</span>
{:else}
	<span class="chip {connection}" role="status">
		<span class="dot" aria-hidden="true"></span>
		{connection === 'connecting' ? m.connection_connecting() : m.connection_offline()}
	</span>
{/if}

<style>
	.holder {
		position: relative;
	}

	.chip {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		height: 1.75rem;
		padding: 0 0.65rem;
		border: none;
		border-radius: var(--radius-s);
		background: none;
		color: var(--muted);
		font-size: 0.85rem;
		white-space: nowrap;
	}

	button.chip:hover,
	button.chip[aria-expanded='true'] {
		background: var(--hover);
	}

	.chip :global(svg) {
		color: var(--accent);
	}

	.speed,
	.limit {
		font-family: var(--mono);
		font-size: 0.8rem;
	}

	.speed {
		color: var(--text);
	}

	.popup {
		position: absolute;
		top: calc(100% + 0.4rem);
		right: 0;
		z-index: 20;
		display: grid;
		gap: 0.6rem;
		width: max-content;
		max-width: 18rem;
		padding: 0.85rem 1rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-m);
		background: var(--surface);
		box-shadow: var(--shadow);
		user-select: text;
	}

	.popup:focus {
		outline: none;
	}

	.title {
		margin: 0;
		font-weight: 600;
	}

	.hint {
		margin: 0;
		color: var(--faint);
		font-size: 0.8rem;
		white-space: normal;
	}

	.dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: var(--quiet);
	}

	.offline {
		color: var(--danger);
	}

	.offline .dot {
		background: var(--danger);
	}
</style>
