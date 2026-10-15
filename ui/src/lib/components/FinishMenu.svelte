<script lang="ts">
	import type { Component } from 'svelte';
	import Check from '@lucide/svelte/icons/check';
	import Flag from '@lucide/svelte/icons/flag';
	import LogOut from '@lucide/svelte/icons/log-out';
	import Moon from '@lucide/svelte/icons/moon';
	import Power from '@lucide/svelte/icons/power';
	import Snowflake from '@lucide/svelte/icons/snowflake';
	import { onMount } from 'svelte';
	import {
		canDo,
		FINISH_CHOICES,
		finishButton,
		finishHint,
		finishName,
		nextFinish,
		soonText
	} from '#lib/finish.js';
	import { LiveFinishing } from '#lib/finish.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { cancelFinish, setFinishOnce, type Finish } from '#lib/window.js';

	// The Downloads page's "When finished" (docs/decisions/0040), in the
	// desktop app: what happens this time only, or Cancel during a countdown.

	const ICONS: Record<Finish, Component> = {
		nothing: Flag,
		quit: LogOut,
		sleep: Moon,
		hibernate: Snowflake,
		shutDown: Power
	};

	const live = new LiveFinishing();
	onMount(() => {
		live.start();
		return () => live.stop();
	});

	let open = $state(false);
	let problem = $state(false);

	const finishing = $derived(live.current);
	const next = $derived(finishing ? nextFinish(finishing) : 'nothing');
	const running = $derived(finishing?.running ?? null);
	const label = $derived(running ? soonText(running.action, live.left ?? 0) : finishButton(next));
	const Icon = $derived(ICONS[running?.action ?? next]);

	/** Closes the list once focus leaves the menu. */
	function focusOut(event: FocusEvent & { currentTarget: HTMLElement }) {
		const target = event.relatedTarget;
		if (!(target instanceof Node && event.currentTarget.contains(target))) {
			open = false;
		}
	}

	async function choose(choice: Finish) {
		problem = !(await live.change(() => setFinishOnce(choice)));
		open = problem;
	}

	async function cancel() {
		open = false;
		await cancelFinish().catch(() => {});
	}

	function keyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			open = false;
		}
	}
</script>

{#if finishing}
	<div class="menu" onfocusout={focusOut}>
		<button
			class:lit={next !== 'nothing' || running}
			class:counting={running}
			aria-haspopup="menu"
			aria-expanded={open}
			aria-label={label}
			title={label}
			onclick={() => (open = !open)}
		>
			<Icon size={15} aria-hidden="true" />
			<span>{label}</span>
		</button>
		{#if open}
			<div class="list" role="menu" tabindex="-1" onkeydown={keyDown}>
				{#if running}
					<button class="item cancel" role="menuitem" onclick={cancel}>
						<span class="mark" aria-hidden="true"></span>
						<span class="names">
							<span>{m.finish_cancel()}</span>
							<span class="hint">{soonText(running.action, live.left ?? 0)}</span>
						</span>
					</button>
					<hr />
				{/if}
				<p class="title">{m.finish_title()}</p>
				{#each FINISH_CHOICES as choice (choice)}
					<button
						class="item"
						role="menuitemradio"
						aria-checked={choice === next}
						disabled={!canDo(finishing, choice)}
						onclick={() => choose(choice)}
					>
						<span class="mark" aria-hidden="true">
							{#if choice === next}<Check size={14} />{/if}
						</span>
						<span class="names">
							<span>{finishName(choice)}</span>
							<span class="hint">{finishHint(finishing, choice)}</span>
						</span>
					</button>
				{/each}
				<p class="note">{problem ? m.finish_failed() : m.finish_once_hint()}</p>
			</div>
		{/if}
	</div>
{/if}

<style>
	.menu {
		position: relative;
	}

	.menu > button {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		height: 2rem;
		padding: 0 0.6rem;
		border-color: transparent;
		background: none;
		white-space: nowrap;
	}

	.menu > button:hover {
		border-color: transparent;
		background: var(--hover);
	}

	.menu > button.lit,
	.menu > button.lit:hover {
		background: var(--selected);
	}

	.menu > button.counting {
		font-variant-numeric: tabular-nums;
	}

	.list {
		position: absolute;
		top: calc(100% + 0.3rem);
		inset-inline-end: 0;
		z-index: 10;
		width: max-content;
		min-width: 16rem;
		max-width: min(22rem, calc(100vw - 2rem));
		padding: 0.3rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-m);
		background: var(--surface);
		box-shadow: var(--shadow);
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
		border-color: transparent;
		border-radius: var(--radius-s);
		background: none;
		text-align: start;
	}

	.item:hover:not(:disabled) {
		border-color: transparent;
		background: var(--hover);
	}

	.item:disabled {
		opacity: 0.5;
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
	}

	.hint {
		color: var(--muted);
		font-size: 0.85em;
	}

	.cancel .names > span:first-child {
		color: var(--danger);
		font-weight: 500;
	}

	hr {
		margin: 0.3rem 0;
		border: none;
		border-top: 1px solid var(--border);
	}

	.note {
		margin: 0.3rem 0.5rem 0.2rem;
		color: var(--faint);
		font-size: 0.8em;
	}

	/* Narrow: the icon only (the button keeps its name for screen readers). */
	@container page (max-width: 56rem) {
		.menu > button span {
			display: none;
		}
	}
</style>
