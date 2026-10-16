<script lang="ts">
	import type { Slot } from '@ottie/api-types/Slot';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash from '@lucide/svelte/icons/trash';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import { useApp } from '#lib/context.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		bodyFromSlot,
		conflicts,
		describeDays,
		describeEffects,
		describeHours,
		describeTarget,
		moved,
		overlaps,
		slotKind,
		slotLabel
	} from '#lib/slots.js';
	import SlotEditor from './SlotEditor.svelte';
	import Switch from './Switch.svelte';
	import WeekView from './WeekView.svelte';

	const { session, ask } = useApp();

	/** The slot in the editor: `null` for a new one, `undefined` when closed. */
	let editing: Slot | null | undefined = $state(undefined);

	let list = $derived(session.slotList);
	let clashes = $derived(conflicts(list));
	let crossings = $derived(overlaps(list));

	async function remove(slot: Slot) {
		if (await ask(m.sched_remove_confirm({ name: slotLabel(slot) }), m.sched_remove())) {
			await session.removeSlot(slot.id);
		}
	}

	async function move(slot: Slot, up: boolean) {
		const ids = moved(list, slot.id, up);
		if (ids) {
			await session.reorderSlots(ids);
		}
	}

	async function toggle(slot: Slot, on: boolean) {
		const why = await session.saveSlot(slot.id, bodyFromSlot(slot, { enabled: on }));
		if (why !== null) {
			session.error = why;
		}
	}

	/** A slot dragged in the week view, saved as it now is. Its editor, if
	 * open, would save the hours it started with: it closes. */
	async function save(slot: Slot) {
		if (editing?.id === slot.id) {
			editing = undefined;
		}
		const why = await session.saveSlot(slot.id, bodyFromSlot(slot));
		if (why !== null) {
			session.error = why;
		}
	}

	/** "Night, Work hours". */
	function names(ids: number[]): string {
		return ids
			.map((id) => session.slots[id])
			.filter((slot) => slot !== undefined)
			.map((slot) => slotLabel(slot))
			.join(', ');
	}
</script>

{#snippet editor(slot: Slot | null)}
	<li class="editing">
		<SlotEditor
			{slot}
			slots={list}
			categories={session.categoryList}
			now={session.now}
			onSave={(body) => session.saveSlot(slot?.id ?? null, body)}
			onClose={() => (editing = undefined)}
		/>
	</li>
{/snippet}

<div class="page">
	<div class="intro">
		<h1>{m.sched_title()}</h1>
		<p>{m.sched_intro()}</p>
		<p class="zone">
			{session.timeZone ? m.sched_time_zone({ zone: session.timeZone }) : m.sched_time_zone_local()}
		</p>
	</div>

	<section class="pane" aria-labelledby="week-title">
		<div class="head">
			<h2 id="week-title">{m.sched_week()}</h2>
		</div>
		<WeekView
			slots={list}
			conflicts={clashes}
			now={session.now}
			onEdit={(slot) => (editing = slot)}
			onMove={(slot) => save(slot)}
		/>
	</section>

	<section class="pane" aria-labelledby="slots-title">
		<div class="head">
			<!-- Only once loaded: "0" while connecting would be wrong. -->
			<h2 id="slots-title">
				{session.connection === 'live' ? m.sched_count({ count: formatCount(list.length) }) : ''}
			</h2>
			<button class="primary" onclick={() => (editing = null)} disabled={editing === null}>
				<Plus size={16} aria-hidden="true" />
				{m.sched_new()}
			</button>
		</div>

		<ul>
			{#if editing === null}
				{@render editor(null)}
			{/if}
			{#each list as slot, i (slot.id)}
				{#if editing?.id === slot.id}
					{@render editor(slot)}
				{:else}
					{@const name = slotLabel(slot)}
					{@const over = clashes[slot.id]}
					{@const crossed = crossings[slot.id]}
					<li class="row" class:off={!slot.enabled}>
						<span class="order">
							<button
								onclick={() => move(slot, true)}
								disabled={i === 0}
								aria-label={m.sched_up()}
								title={m.sched_up()}
							>
								<ArrowUp size={14} />
							</button>
							<button
								onclick={() => move(slot, false)}
								disabled={i === list.length - 1}
								aria-label={m.sched_down()}
								title={m.sched_down()}
							>
								<ArrowDown size={14} />
							</button>
						</span>
						<span class="mark {slotKind(slot)}" aria-hidden="true"></span>
						<div class="who">
							<span class="name">
								<!-- Without a name, when it is stands in for one. -->
								<span dir="auto">{slot.name.trim() ? name : describeHours(slot)}</span>
								{#if slot.active}
									<span class="badge now">{m.sched_now()}</span>
								{:else if !slot.enabled}
									<span class="badge">{m.sched_off()}</span>
								{/if}
							</span>
							<span class="when">
								{[
									describeDays(slot),
									slot.name.trim() ? describeHours(slot) : null,
									describeTarget(slot, session.categories)
								]
									.filter((part) => part !== null)
									.join(' · ')}
							</span>
							<!-- Overridden says more than overlaps: one or the other. -->
							{#if over}
								<span class="conflict">
									<TriangleAlert size={13} aria-hidden="true" />
									{m.sched_conflict({ names: names(over) })}
								</span>
							{:else if crossed}
								<span class="conflict">
									<TriangleAlert size={13} aria-hidden="true" />
									{m.sched_overlaps({ names: names(crossed) })}
								</span>
							{/if}
						</div>
						<div class="effects">
							{#each describeEffects(slot) as effect (effect)}
								<span class="chip">{effect}</span>
							{/each}
						</div>
						<span class="end">
							<Switch
								on={slot.enabled}
								label={m.sched_switch()}
								onChange={(on) => toggle(slot, on)}
							/>
							<span class="actions">
								<button
									onclick={() => (editing = slot)}
									aria-label={m.sched_edit_label({ name })}
									title={m.sched_edit()}
								>
									<Pencil size={15} />
								</button>
								<button
									onclick={() => remove(slot)}
									aria-label={m.sched_remove_label({ name })}
									title={m.sched_remove()}
								>
									<Trash size={15} />
								</button>
							</span>
						</span>
					</li>
				{/if}
			{:else}
				{#if editing !== null && session.connection === 'live'}
					<li class="empty">{m.sched_empty()}</li>
				{/if}
			{/each}
		</ul>
	</section>
</div>

<style>
	.page {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 0.75rem;
		min-height: 0;
		padding: 0.25rem 1rem 1rem;
		overflow: auto;
	}

	.intro {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		padding: 0.25rem;
	}

	h1 {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 600;
	}

	.intro p {
		max-width: 46rem;
		margin: 0;
		color: var(--muted);
	}

	.intro .zone {
		color: var(--faint);
		font-size: 0.85rem;
	}

	.pane {
		flex: none;
		border: 1px solid var(--edge);
		border-radius: var(--radius-l);
		background: var(--surface);
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 1rem;
		min-height: 2.25rem;
		padding: 0.6rem 0.75rem 0.6rem 1.15rem;
		border-bottom: 1px solid var(--border-soft);
	}

	h2 {
		margin: 0;
		color: var(--muted);
		font-size: 0.95rem;
		font-weight: 400;
	}

	.head button {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin-inline-start: auto;
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		border-bottom: 1px solid var(--border-soft);
	}

	li:last-child {
		border-bottom: none;
	}

	.empty {
		padding: 1rem 1.15rem;
		color: var(--muted);
	}

	/* One line per slot on wide panes; the parts wrap as it narrows. */
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 1rem;
		padding: 0.65rem 0.75rem 0.65rem 0.6rem;
	}

	.row.off .who,
	.row.off .effects {
		opacity: 0.6;
	}

	.order {
		display: flex;
		flex-direction: column;
	}

	.order button,
	.actions button {
		display: grid;
		place-items: center;
		width: 1.6rem;
		height: 1.3rem;
		padding: 0;
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	.actions button {
		width: 2rem;
		height: 2rem;
	}

	.order button:hover:not(:disabled),
	.actions button:hover {
		background: var(--hover);
		color: var(--text);
	}

	.order button:disabled {
		opacity: 0.3;
	}

	/* The slot's colour in the week view. */
	.mark {
		flex: none;
		align-self: stretch;
		width: 4px;
		border-radius: 2px;
		background: var(--accent);
	}

	.mark.stop {
		background: var(--danger);
	}

	.mark.allow {
		background: var(--ok);
	}

	/* 12rem: beside the arrows and the mark even on a phone. */
	.who {
		display: grid;
		flex: 1 1 12rem;
		gap: 0.1rem;
		min-width: 0;
	}

	.name {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 600;
	}

	.badge {
		padding: 0 0.5rem;
		border-radius: 999px;
		background: var(--well);
		color: var(--muted);
		font-size: 0.72rem;
		font-weight: 400;
		line-height: 1.2rem;
	}

	.badge.now {
		background: var(--ok-bg);
		color: var(--ok);
	}

	.when {
		color: var(--muted);
		font-size: 0.85rem;
	}

	.conflict {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		color: var(--warn);
		font-size: 0.8rem;
	}

	.effects {
		display: flex;
		flex: 1 1 14rem;
		flex-wrap: wrap;
		gap: 0.3rem;
	}

	.chip {
		padding: 0 0.55rem;
		border: 1px solid var(--border-soft);
		border-radius: 999px;
		background: var(--well);
		font-size: 0.8rem;
		line-height: 1.4rem;
	}

	/* The switch and the buttons, at the end even on a line of their own. */
	.end {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin-inline-start: auto;
	}

	.actions {
		display: flex;
		gap: 0.15rem;
	}

	@media (max-width: 40rem) {
		.page {
			padding: 0.25rem 0.5rem 0.75rem;
		}
	}
</style>
