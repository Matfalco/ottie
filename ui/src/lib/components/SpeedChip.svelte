<script lang="ts">
	import Gauge from '@lucide/svelte/icons/gauge';
	import { formatSpeed } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { Connection } from '#lib/session.svelte.js';

	interface Props {
		/** All downloads together, in bytes per second. */
		speed: number;
		connection: Connection;
	}

	let { speed, connection }: Props = $props();
</script>

<!-- The total speed in the title bar; while the daemon can't be reached, why
     instead. (The global limit joins it with the settings.) -->
{#if connection === 'live'}
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
	.chip {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		height: 1.75rem;
		padding: 0 0.65rem;
		border-radius: var(--radius-s);
		color: var(--muted);
		font-size: 0.85rem;
		white-space: nowrap;
	}

	.chip :global(svg) {
		color: var(--accent);
	}

	.speed {
		color: var(--text);
		font-family: var(--mono);
		font-size: 0.8rem;
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
