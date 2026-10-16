<script lang="ts">
	import type { Category } from '@ottie/api-types/Category';
	import type { Day } from '@ottie/api-types/Day';
	import type { Slot } from '@ottie/api-types/Slot';
	import type { SlotBody } from '@ottie/api-types/SlotBody';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import { categoryName } from '#lib/categories.js';
	import { unfold } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { toField } from '#lib/schedule.js';
	import { SPEED_UNITS, unitLabel } from '#lib/settings.js';
	import { DAYS, bodyOf, dayName, draftOf, meeting, slotLabel, slotOfBody } from '#lib/slots.js';
	import Switch from './Switch.svelte';

	interface Props {
		/** The slot to edit; null for a new one. */
		slot: Slot | null;
		/** Every slot, in the order of the list: to say which it overlaps. */
		slots: Slot[];
		/** General first, then by name. */
		categories: Category[];
		now: number;
		/** Resolves to null once saved, else to why not. */
		onSave: (body: SlotBody) => Promise<string | null>;
		onClose: () => void;
	}

	let { slot, slots, categories, now, onSave, onClose }: Props = $props();

	// The form starts from the slot; it's a copy, edited freely.
	// svelte-ignore state_referenced_locally
	let draft = $state(draftOf(slot, toField(now).slice(0, 10)));
	let saving = $state(false);
	/** The others it would overlap, as it's being set. */
	let crossed = $derived.by(() => {
		const made = bodyOf(draft);
		return 'body' in made ? meeting(slotOfBody(made.body, slot?.id ?? null), slots) : [];
	});
	let problem: string | null = $state(null);

	function toggleDay(day: Day) {
		draft.days = draft.days.includes(day)
			? draft.days.filter((each) => each !== day)
			: [...draft.days, day];
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		const made = bodyOf(draft);
		if ('problem' in made) {
			problem = made.problem;
			return;
		}
		saving = true;
		problem = await onSave(made.body);
		saving = false;
		if (problem === null) {
			onClose();
		}
	}
</script>

<!-- Opens in the list, in place of the slot's row (or at the top for a new
     one). Escape closes it without saving. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form
	onsubmit={submit}
	onkeydown={(event) => event.key === 'Escape' && onClose()}
	aria-labelledby="slot-editor-title"
>
	<h2 id="slot-editor-title">
		{slot ? m.sched_editing({ name: slotLabel(slot) }) : m.sched_new()}
	</h2>

	<div class="fields">
		<label>
			<span>{m.sched_name()}</span>
			<!-- svelte-ignore a11y_autofocus -->
			<input bind:value={draft.name} dir="auto" autofocus />
			<small>{m.sched_name_hint()}</small>
		</label>

		<label>
			<span>{m.sched_applies()}</span>
			<select bind:value={draft.category}>
				<option value={null}>{m.slot_whole_queue()}</option>
				{#each categories as category (category.id)}
					<option value={category.id}>{categoryName(category)}</option>
				{/each}
			</select>
		</label>

		<fieldset class="wide">
			<legend>{m.sched_when()}</legend>
			<div class="segments">
				<label class:chosen={!draft.once}>
					<input type="radio" bind:group={draft.once} value={false} />
					{m.sched_weekly()}
				</label>
				<label class:chosen={draft.once}>
					<input type="radio" bind:group={draft.once} value={true} />
					{m.sched_once()}
				</label>
			</div>
			{#if draft.once}
				<label class="inline">
					<span class="sub">{m.sched_date()}</span>
					<input type="date" bind:value={draft.date} />
				</label>
			{:else}
				<div class="days" role="group" aria-label={m.sched_days()}>
					{#each DAYS as day (day)}
						<button
							type="button"
							class="day"
							aria-pressed={draft.days.includes(day)}
							title={dayName(day, 'long')}
							onclick={() => toggleDay(day)}
						>
							{dayName(day, 'short')}
						</button>
					{/each}
				</div>
			{/if}
			<div class="hours">
				<label class="inline">
					<span class="sub">{m.sched_from()}</span>
					<input type="time" bind:value={draft.from} required />
				</label>
				<label class="inline">
					<span class="sub">{m.sched_to()}</span>
					<input type="time" bind:value={draft.to} required />
				</label>
			</div>
			<small>{m.sched_hours_hint()}</small>
			{#if crossed.length > 0}
				<p class="overlap" role="status" in:unfold out:unfold>
					<TriangleAlert size={14} aria-hidden="true" />
					{m.sched_editor_overlaps({
						names: crossed.map((other) => slotLabel(other)).join(', ')
					})}
				</p>
			{/if}
		</fieldset>

		<fieldset class="wide">
			<legend>{m.sched_downloads()}</legend>
			<div class="segments">
				<label class:chosen={draft.downloads === null}>
					<input type="radio" bind:group={draft.downloads} value={null} />
					{m.sched_leave()}
				</label>
				<label class:chosen={draft.downloads === 'allow'}>
					<input type="radio" bind:group={draft.downloads} value="allow" />
					{m.sched_allow()}
				</label>
				<label class:chosen={draft.downloads === 'stop'}>
					<input type="radio" bind:group={draft.downloads} value="stop" />
					{m.sched_stop()}
				</label>
			</div>
			<small>
				{draft.downloads === null
					? m.sched_leave_hint()
					: draft.downloads === 'allow'
						? m.sched_allow_hint()
						: m.sched_stop_hint()}
			</small>
		</fieldset>

		{#if draft.downloads === 'stop'}
			<div class="wide pair">
				<fieldset>
					<legend>{m.sched_running()}</legend>
					<div class="segments">
						<label class:chosen={!draft.letFinish}>
							<input type="radio" bind:group={draft.letFinish} value={false} />
							{m.sched_pause_them()}
						</label>
						<label class:chosen={draft.letFinish}>
							<input type="radio" bind:group={draft.letFinish} value={true} />
							{m.sched_let_finish()}
						</label>
					</div>
				</fieldset>

				<fieldset>
					<legend>{m.sched_at_end()}</legend>
					<div class="segments">
						<label class:chosen={!draft.stayPaused}>
							<input type="radio" bind:group={draft.stayPaused} value={false} />
							{m.sched_carry_on()}
						</label>
						<label class:chosen={draft.stayPaused}>
							<input type="radio" bind:group={draft.stayPaused} value={true} />
							{m.sched_stay_paused()}
						</label>
					</div>
					<small>{draft.stayPaused ? m.sched_stay_paused_hint() : m.sched_carry_on_hint()}</small>
				</fieldset>
			</div>
		{:else}
			<div class="limit">
				<div class="switch-row">
					<Switch
						on={draft.limits}
						label={m.sched_set_limit()}
						onChange={(on) => (draft.limits = on)}
					/>
					<span>{m.sched_set_limit()}</span>
				</div>
				{#if draft.limits}
					<span class="row" in:unfold out:unfold>
						<input bind:value={draft.speed} inputmode="decimal" aria-label={m.sched_set_limit()} />
						<select bind:value={draft.unit} aria-label={m.sched_set_limit()}>
							{#each SPEED_UNITS as option (option)}
								<option value={option}>{unitLabel(option)}</option>
							{/each}
						</select>
					</span>
					<small in:unfold out:unfold>{m.sched_limit_hint()}</small>
				{/if}
			</div>

			<div class="limit">
				<div class="switch-row">
					<Switch on={draft.caps} label={m.sched_set_max()} onChange={(on) => (draft.caps = on)} />
					<span>{m.sched_set_max()}</span>
				</div>
				{#if draft.caps}
					<!-- Text, not type="number": Svelte would bind a number. -->
					<input
						bind:value={draft.max}
						inputmode="numeric"
						aria-label={m.sched_set_max()}
						in:unfold
						out:unfold
					/>
				{/if}
			</div>
		{/if}
	</div>

	{#if problem}
		<p class="problem" role="alert" in:unfold out:unfold>{problem}</p>
	{/if}

	<div class="buttons">
		<div class="switch-row">
			<Switch on={draft.enabled} label={m.sched_switch()} onChange={(on) => (draft.enabled = on)} />
			<span>{m.sched_switch()}</span>
		</div>
		<button type="button" onclick={onClose}>{m.sched_cancel()}</button>
		<button class="primary" disabled={saving}>{m.sched_save()}</button>
	</div>
</form>

<style>
	form {
		display: grid;
		gap: 0.9rem;
		padding: 1rem 1.25rem 1.1rem;
		background: var(--well);
	}

	h2 {
		margin: 0;
		font-size: 1rem;
	}

	.fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
		gap: 0.9rem 1.25rem;
	}

	.wide {
		grid-column: 1 / -1;
	}

	/* Two choices side by side, each as wide as its segments need. */
	.pair {
		display: flex;
		flex-wrap: wrap;
		gap: 0.9rem 1.25rem;
	}

	.pair fieldset {
		flex: 0 1 28rem;
	}

	label,
	fieldset,
	.limit {
		display: grid;
		align-content: start;
		gap: 0.35rem;
		min-width: 0;
		margin: 0;
		padding: 0;
		border: none;
	}

	label > span:first-child,
	legend,
	.limit .switch-row > span {
		font-size: 0.85rem;
		font-weight: 600;
	}

	legend {
		margin-bottom: 0.35rem;
		padding: 0;
	}

	.sub {
		color: var(--muted);
		font-size: 0.8rem;
		font-weight: 400 !important;
	}

	.inline {
		display: grid;
		gap: 0.2rem;
	}

	small {
		color: var(--muted);
		font-size: 0.8rem;
	}

	/* One height for every field, whatever its kind. */
	input:not([type='radio']),
	select {
		height: 2.25rem;
		padding-block: 0;
	}

	.hours,
	.days,
	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem 1rem;
	}

	.days {
		gap: 0.3rem;
	}

	.row {
		gap: 0.4rem;
		flex-wrap: nowrap;
	}

	.row input {
		flex: 1 1 0;
		width: 0;
		min-width: 0;
	}

	/* A day drawn as a chip that's pressed or not. */
	.day {
		min-width: 3rem;
		border-radius: 999px;
	}

	.day[aria-pressed='true'] {
		border-color: var(--accent);
		background: var(--selected);
	}

	/* Holds its hidden radio button: without it, the button would hang from
	   the window and stretch it. */
	.segments label {
		position: relative;
		display: grid;
		flex: 1;
		place-items: center;
		align-content: center;
		min-height: 1.95rem;
		padding: 0.25rem 0.75rem;
		text-align: center;
		color: var(--muted);
		font-size: 0.9rem;
		font-weight: 400;
		cursor: pointer;
		border-radius: 999px;
	}

	.segments label:hover {
		background: var(--hover);
	}

	/* Only when reached from the keyboard: a click shows the choice by the
	   highlight alone. */
	.segments label:has(input:focus-visible) {
		outline: 2px solid var(--focus);
		outline-offset: -2px;
	}

	.segments input {
		position: absolute;
		opacity: 0;
		pointer-events: none;
	}

	.switch-row {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}

	.problem {
		margin: 0;
		color: var(--danger);
	}

	.overlap {
		display: flex;
		align-items: flex-start;
		gap: 0.35rem;
		margin: 0.2rem 0 0;
		color: var(--warn);
		font-size: 0.85rem;
	}

	.overlap :global(svg) {
		flex: none;
		margin-top: 0.15rem;
	}

	.buttons {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	.buttons .switch-row {
		margin-inline-end: auto;
	}

	/* Radio buttons drawn as one row of segments (as in the detail panel). */
	.segments {
		display: flex;
		max-width: 28rem;
		overflow: hidden;
		gap: 0.15rem;
		padding: 0.2rem;
		border: none;
		border-radius: 999px;
		background: var(--sunken);
		box-shadow: var(--sunken-shadow);
	}

	.segments label + label {
		border-inline-start: none;
	}

	.segments label.chosen {
		color: var(--text);
		font-weight: 500;
		background: var(--raised);
		box-shadow: var(--raised-shadow);
	}
</style>
