<script lang="ts">
	import CalendarClock from '@lucide/svelte/icons/calendar-clock';
	import Gauge from '@lucide/svelte/icons/gauge';
	import { resolve } from '$app/paths';
	import { formatSpeed } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { Connection } from '#lib/session.svelte.js';
	import { shortLabel, type queueNow } from '#lib/slots.js';
	import LimitControl from './LimitControl.svelte';
	import RatePresets from './RatePresets.svelte';

	interface Props {
		/** All downloads together, in bytes per second. */
		speed: number;
		/** The total limit in bytes per second (0 = none); null until known. */
		limit: number | null;
		/** What the time slots do to all downloads right now. */
		schedule: ReturnType<typeof queueNow>;
		connection: Connection;
		/** Saves a new total limit; resolves to false if it couldn't. */
		onLimit: (bytesPerSec: number) => Promise<boolean>;
		/** The time slot's chip as its icon alone: the title bar is short of room. */
		brief?: boolean;
		/** The title bar is still short of room: the chip shortens to fit,
		 * cut with "…" (the whole in its tooltip), so it and its popup stay
		 * inside the window. */
		squeezed?: boolean;
	}

	let {
		speed,
		limit,
		schedule,
		connection,
		onLimit,
		brief = false,
		squeezed = false
	}: Props = $props();
	let open = $state(false);
	let chip: HTMLButtonElement | undefined = $state();

	/** The slot at work on all downloads, if one is: stopping them, or limiting. */
	let note = $derived(
		schedule.stoppedBy
			? m.chip_stopped({ name: shortLabel(schedule.stoppedBy) })
			: schedule.limit
				? shortLabel(schedule.limit.slot)
				: null
	);
	/** A slot's limit replaces the setting's while it lasts. */
	let shown = $derived(schedule.limit?.bytesPerSec ?? limit);
	const label = $derived(
		[
			shown
				? m.chip_speed_limited({ speed: formatSpeed(speed), limit: formatSpeed(shown) })
				: m.chip_speed({ speed: formatSpeed(speed) }),
			schedule.limit ? m.chip_slot_limit({ name: shortLabel(schedule.limit.slot) }) : null,
			schedule.stoppedBy ? m.chip_stopped({ name: shortLabel(schedule.stoppedBy) }) : null
		]
			.filter((part) => part !== null)
			.join(' · ')
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
	<div class="holder" class:squeezed onfocusout={focusOut} onkeydown={keyDown} role="presentation">
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
			{#if shown}
				<span class="limit">/ {formatSpeed(shown)}</span>
			{/if}
		</button>
		{#if note}
			<!-- A time slot is at work: says which, and leads to the schedule. -->
			<a class="slot" class:brief href={resolve('/schedule')} title={label} aria-label={label}>
				<CalendarClock size={13} aria-hidden="true" />
				<span class="note">{note}</span>
			</a>
		{/if}
		{#if open}
			<!-- Focusable, so a click inside doesn't count as leaving it. -->
			<div class="popup" role="dialog" aria-label={m.limit_total()} tabindex="-1">
				<p class="title">{m.limit_total()}</p>
				<LimitControl {limit} onChange={onLimit} />
				<RatePresets {limit} onChange={onLimit} />
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
		display: flex;
		align-items: center;
	}

	.holder.squeezed,
	.squeezed .chip,
	.squeezed .chip span {
		min-width: 0;
	}

	.squeezed .chip span {
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.squeezed .chip :global(svg) {
		flex: none;
	}

	/* Short of room: the icon alone, the rest in its tooltip. */
	.slot.brief .note {
		display: none;
	}

	.slot {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		height: 1.5rem;
		padding: 0 0.5rem;
		border-radius: 999px;
		background: var(--warn-bg);
		color: var(--warn);
		font-size: 0.75rem;
		text-decoration: none;
		white-space: nowrap;
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
		inset-inline-end: 0;
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

	/* Phones: across the screen, under the title bar (fixed, so the title
	   bar's middle, which cuts off what's too wide, doesn't cut it). */
	@media (max-width: 40rem) {
		.popup {
			position: fixed;
			top: 3.25rem;
			inset-inline-end: 0.5rem;
			inset-inline-start: 0.5rem;
			width: auto;
			max-width: none;
		}
	}
</style>
