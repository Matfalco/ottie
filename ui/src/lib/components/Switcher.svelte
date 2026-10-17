<script lang="ts">
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Server from '@lucide/svelte/icons/server';
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { drop } from '#lib/moving.svelte.js';
	import { LiveOthers } from '#lib/others.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	// Beside the logo, once there's another computer's Ottie to go to
	// (docs/decisions/0053): which one this is, and the others.

	interface Props {
		/** The title bar is folded: the icon only. */
		folded?: boolean;
	}

	let { folded = false }: Props = $props();

	const others = new LiveOthers();
	onMount(() => others.start());

	let open = $state(false);
	let problem: string | null = $state(null);
	const current = $derived(others.entries.find((entry) => entry.current));
	const label = $derived(
		current ? m.others_switch_label({ name: current.name }) : m.others_switch_title()
	);

	/** Closes the list once focus leaves the menu. */
	function focusOut(event: FocusEvent & { currentTarget: HTMLElement }) {
		const target = event.relatedTarget;
		if (!(target instanceof Node && event.currentTarget.contains(target))) {
			open = false;
		}
	}

	async function choose(id: number | null, isCurrent: boolean) {
		if (isCurrent) {
			open = false;
			return;
		}
		problem = await others.open(id);
		open = problem !== null;
	}

	function keyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			open = false;
		}
	}
</script>

{#if others.entries.length > 1 && current}
	<div class="menu" onfocusout={focusOut}>
		<button
			class="where"
			class:folded
			aria-haspopup="menu"
			aria-expanded={open}
			aria-label={label}
			title={m.others_switch_title()}
			onclick={() => {
				problem = null;
				open = !open;
			}}
		>
			<Server size={15} aria-hidden="true" />
			<span class="name" dir="auto">{current.name}</span>
			<ChevronDown size={14} aria-hidden="true" />
		</button>
		{#if open}
			<div class="list" role="menu" tabindex="-1" onkeydown={keyDown} in:drop out:drop>
				<p class="title">{m.others_title()}</p>
				{#each others.entries as entry (entry.id ?? 'here')}
					<button
						class="item"
						role="menuitemradio"
						aria-checked={entry.current}
						disabled={others.busy}
						onclick={() => void choose(entry.id, entry.current)}
					>
						<span class="mark" aria-hidden="true">
							{#if entry.current}<Check size={14} />{/if}
						</span>
						<span class="names">
							<span dir="auto">{entry.name}</span>
							<span class="hint" dir="auto">{entry.address}</span>
						</span>
					</button>
				{/each}
				{#if problem}
					<p class="problem" role="alert">{problem}</p>
				{:else if others.busy}
					<p class="note">{m.others_checking()}</p>
				{/if}
				<hr />
				<a
					class="item manage"
					href="{resolve('/settings')}#set-connection"
					onclick={() => (open = false)}
				>
					<span class="mark" aria-hidden="true"></span>
					{m.others_manage()}
				</a>
			</div>
		{/if}
	</div>
{/if}

<style>
	.menu {
		position: relative;
		flex: none;
	}

	.where {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		height: 2rem;
		max-width: 12rem;
		padding: 0 0.6rem;
		border-color: transparent;
		border-radius: var(--radius-pill);
		background: var(--sunken);
		white-space: nowrap;
	}

	.where:hover {
		border-color: transparent;
		background: var(--hover);
	}

	.name {
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Through a class, as the title bar folds (TitleBar's `level`). */
	.folded .name {
		display: none;
	}

	.list {
		position: absolute;
		top: calc(100% + 0.3rem);
		inset-inline-start: 0;
		z-index: 20;
		width: max-content;
		min-width: 16rem;
		max-width: min(22rem, calc(100vw - 2rem));
		padding: 0.3rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-m);
		background: var(--surface);
		box-shadow: var(--shadow);
		font-size: 0.875rem;
		font-weight: 400;
	}

	.title {
		margin: 0.3rem 0.5rem 0.2rem;
		color: var(--muted);
		font-size: 0.8em;
		font-weight: 500;
	}

	.item {
		display: flex;
		align-items: flex-start;
		gap: 0.4rem;
		width: 100%;
		padding: 0.4rem 0.5rem;
		border: 1px solid transparent;
		border-radius: var(--radius-s);
		background: none;
		color: var(--text);
		text-align: start;
		text-decoration: none;
	}

	.item:hover:not(:disabled) {
		border-color: transparent;
		background: var(--hover);
	}

	.item:disabled {
		opacity: 0.6;
	}

	.mark {
		display: flex;
		flex: none;
		width: 14px;
		padding-top: 0.15rem;
		color: var(--accent);
	}

	.names {
		display: grid;
		gap: 0.1rem;
		min-width: 0;
	}

	.names span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.hint {
		color: var(--muted);
		font-family: var(--mono);
		font-size: 0.8em;
	}

	hr {
		margin: 0.3rem 0;
		border: none;
		border-top: 1px solid var(--border);
	}

	.note,
	.problem {
		margin: 0.3rem 0.5rem 0.2rem;
		font-size: 0.85em;
	}

	.note {
		color: var(--faint);
	}

	.problem {
		color: var(--danger);
	}
</style>
