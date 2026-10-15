<script lang="ts">
	import type { Slot } from '@ottie/api-types/Slot';
	import { m } from '#lib/paraglide/messages.js';
	import {
		DAYS,
		dayName,
		describeEffects,
		describeHours,
		dragged,
		formatHour,
		mondayOf,
		slotKind,
		slotLabel,
		weekLayout,
		type Drag
	} from '#lib/slots.js';

	interface Props {
		/** In the order of the list. */
		slots: Slot[];
		/** Lower slots overridden by higher ones, by ID. */
		conflicts: Record<number, number[]>;
		now: number;
		/** A click on a slot (or Enter). */
		onEdit: (slot: Slot) => void;
		/** A slot dragged to new hours or days: saves it. */
		onMove: (slot: Slot) => Promise<void>;
	}

	let { slots, conflicts, now, onEdit, onMove }: Props = $props();

	const DAY_MINUTES = 24 * 60;
	const HOURS = [0, 3, 6, 9, 12, 15, 18, 21];
	/** The shortest block a name fits in; shorter ones keep their tooltip. */
	const LABELLED = 60;
	/** How far the pointer goes before a press becomes a drag, in pixels. */
	const SLOP = 4;

	/** A drag under way: the slot as it was, what's dragged, where the
	 * pointer started, and the slot as it would be now. */
	interface Dragging {
		slot: Slot;
		drag: Drag;
		x: number;
		y: number;
		moved: boolean;
		shown: Slot;
	}

	let dragging: Dragging | null = $state(null);
	/** A dragged slot being saved: shown where it was dropped meanwhile. */
	let saving: Slot | null = $state(null);
	/** The click that ends a drag isn't one. */
	let dropped = false;
	let week: HTMLElement | undefined = $state();

	let shown = $derived.by(() => {
		const over = dragging?.shown ?? saving;
		return over ? slots.map((slot) => (slot.id === over.id ? over : slot)) : slots;
	});
	let monday = $derived(mondayOf(new Date(now)));
	let blocks = $derived(weekLayout(shown, monday));
	let byId = $derived(Object.fromEntries(shown.map((slot) => [slot.id, slot])));
	let today = $derived((new Date(now).getDay() + 6) % 7);
	let minute = $derived(new Date(now).getHours() * 60 + new Date(now).getMinutes());

	function percent(minutes: number): string {
		return `${(minutes / DAY_MINUTES) * 100}%`;
	}

	function begin(event: PointerEvent, slot: Slot, drag: Drag) {
		if (event.button !== 0) {
			return;
		}
		// An edge is inside its block: the block mustn't start a move too.
		event.stopPropagation();
		event.preventDefault();
		dropped = false;
		dragging = { slot, drag, x: event.clientX, y: event.clientY, moved: false, shown: slot };
	}

	/** Follows the pointer anywhere on the page: a block that changes day
	 * is drawn anew in another column. */
	function follow(event: PointerEvent) {
		const columns = week?.querySelectorAll<HTMLElement>('.column');
		const first = columns?.[0]?.getBoundingClientRect();
		const second = columns?.[1]?.getBoundingClientRect();
		if (!dragging || !first || !second) {
			return;
		}
		const dx = event.clientX - dragging.x;
		const dy = event.clientY - dragging.y;
		if (!dragging.moved && Math.hypot(dx, dy) < SLOP) {
			return;
		}
		dragging.moved = true;
		const minutes = (dy / first.height) * DAY_MINUTES;
		const days = Math.round(dx / (second.left - first.left));
		dragging.shown = dragged(dragging.slot, dragging.drag, minutes, days);
	}

	async function drop() {
		const done = dragging;
		dragging = null;
		if (!done?.moved) {
			return;
		}
		// The click that follows the release, if one does, comes before this.
		dropped = true;
		setTimeout(() => (dropped = false));
		const after = done.shown;
		const before = done.slot;
		const same =
			after.from === before.from &&
			after.to === before.to &&
			after.date === before.date &&
			after.days.join() === before.days.join();
		if (same) {
			return;
		}
		saving = after;
		await onMove(after);
		saving = null;
	}

	function click(slot: Slot) {
		if (!dropped) {
			onEdit(slot);
		}
	}
</script>

<svelte:window
	onpointermove={follow}
	onpointerup={drop}
	onpointercancel={() => (dragging = null)}
	onkeydown={(event) => event.key === 'Escape' && (dragging = null)}
/>

<!-- The slots that are on, over this week: a column per day, midnight at
     the top. Where several share a day they sit side by side, the higher
     one in the list on the left. A slot can be dragged to other hours or
     days, or by its top or bottom edge to start or end at another time; a
     click opens it in the editor. -->
<div
	class="week"
	class:dragging={dragging?.moved}
	role="group"
	aria-label={m.sched_week_label()}
	bind:this={week}
>
	<span></span>
	{#each DAYS as day, i (day)}
		<span class="day" class:today={i === today}>{dayName(day, 'short')}</span>
	{/each}

	<div class="hours" aria-hidden="true">
		{#each HOURS as hour (hour)}
			<span style:top={percent(hour * 60)}>{formatHour(hour)}</span>
		{/each}
	</div>
	{#each DAYS as day, i (day)}
		<div class="column" class:today={i === today}>
			{#each blocks.filter((block) => block.day === i) as block (`${block.slot}-${block.first}`)}
				{@const slot = byId[block.slot]}
				{#if slot}
					{@const moving = dragging?.moved === true && dragging.slot.id === slot.id}
					<button
						type="button"
						class="block {slotKind(slot)}"
						class:active={slot.active}
						class:moving
						class:overridden={conflicts[slot.id] !== undefined}
						style:top={percent(block.start)}
						style:height={percent(block.end - block.start)}
						style:inset-inline-start={`${(block.lane / block.lanes) * 100}%`}
						style:width={`${100 / block.lanes}%`}
						title={[slotLabel(slot), describeHours(slot), ...describeEffects(slot)].join(' · ')}
						aria-label={m.sched_block_label({ name: slotLabel(slot), hours: describeHours(slot) })}
						onpointerdown={(event) => begin(event, slot, 'move')}
						onclick={() => click(slot)}
					>
						{#if block.end - block.start >= LABELLED}
							<!-- While dragged, its new hours. -->
							<span class="label" dir="auto">{moving ? describeHours(slot) : slotLabel(slot)}</span>
						{/if}
						{#if block.first}
							<span
								class="edge top"
								aria-hidden="true"
								onpointerdown={(event) => begin(event, slot, 'start')}
							></span>
						{/if}
						{#if block.last}
							<span
								class="edge bottom"
								aria-hidden="true"
								onpointerdown={(event) => begin(event, slot, 'end')}
							></span>
						{/if}
					</button>
				{/if}
			{/each}
			{#if i === today}
				<span class="now" style:top={percent(minute)}></span>
			{/if}
		</div>
	{/each}
</div>
<p class="hint">{m.sched_week_hint()}</p>

<style>
	.week {
		display: grid;
		user-select: none;
		grid-template-columns: 2.6rem repeat(7, minmax(0, 1fr));
		grid-template-rows: auto 24rem;
		gap: 0.3rem 0.25rem;
		padding: 0.75rem 1rem 1rem 0.5rem;
	}

	.day {
		color: var(--muted);
		font-size: 0.8rem;
		text-align: center;
	}

	.day.today {
		color: var(--text);
		font-weight: 600;
	}

	.hours {
		position: relative;
	}

	.hours span {
		position: absolute;
		inset-inline-end: 0.3rem;
		color: var(--faint);
		font-size: 0.7rem;
		transform: translateY(-50%);
	}

	.hours span:first-child {
		transform: none;
	}

	/* A line every three hours, in the column's background. */
	.column {
		position: relative;
		overflow: hidden;
		border-radius: var(--radius-s);
		background:
			repeating-linear-gradient(
				transparent 0 calc(12.5% - 1px),
				var(--border-soft) calc(12.5% - 1px) 12.5%
			),
			var(--well);
	}

	.column.today {
		outline: 1px solid var(--border);
	}

	/* Each kind's colour, and a soft fill of it that suits the theme. */
	.block {
		position: absolute;
		/* A button centres what it holds: the name goes at the top. */
		display: flex;
		flex-direction: column;
		justify-content: flex-start;
		overflow: hidden;
		padding: 0.05rem 0.25rem;
		border: 1px solid transparent;
		border-inline-start: 3px solid var(--hue);
		border-radius: 3px;
		--hue: var(--accent);
		--fill: color-mix(in srgb, var(--accent) 20%, var(--surface));
		background: var(--fill);
		color: var(--text);
		font: inherit;
		text-align: start;
		cursor: grab;
		touch-action: none;
	}

	.block:hover {
		border-color: var(--hue);
	}

	.block.stop {
		--hue: var(--danger);
		--fill: var(--danger-bg);
	}

	.block.allow {
		--hue: var(--ok);
		--fill: var(--ok-bg);
	}

	.block.active {
		border-color: var(--hue);
		background: color-mix(in srgb, var(--hue) 22%, var(--fill));
	}

	/* A higher slot overrides it somewhere. */
	.block.overridden {
		border-style: dashed;
		border-color: var(--warn);
		border-inline-start-style: solid;
	}

	.block.moving {
		z-index: 1;
		box-shadow: var(--shadow);
	}

	.dragging,
	.dragging .block {
		cursor: grabbing;
	}

	/* The top and bottom edges, to drag the start or the end. */
	.edge {
		position: absolute;
		inset-inline-end: 0;
		inset-inline-start: 0;
		height: 6px;
		cursor: ns-resize;
	}

	.edge.top {
		top: 0;
	}

	.edge.bottom {
		bottom: 0;
	}

	.hint {
		margin: 0;
		padding: 0 1rem 0.9rem 3.35rem;
		color: var(--muted);
		font-size: 0.8rem;
	}

	.label {
		display: block;
		overflow: hidden;
		font-size: 0.7rem;
		line-height: 1.2;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.now {
		position: absolute;
		inset-inline-end: 0;
		inset-inline-start: 0;
		height: 2px;
		background: var(--accent);
	}
</style>
