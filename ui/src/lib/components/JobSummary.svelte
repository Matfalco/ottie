<script lang="ts">
	import type { Job } from '@ottie/api-types/Job';
	import Play from '@lucide/svelte/icons/play';
	import Trash from '@lucide/svelte/icons/trash';
	import type { Action } from '#lib/bulk.js';
	import { displayName } from '#lib/format.js';
	import { leave } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { canResume, describe } from '#lib/status.js';
	import FileIcon from './FileIcon.svelte';
	import { useApp } from '#lib/context.js';

	interface Props {
		job: Job;
		/** Its key in the dashboard's slides (`shuffle`): the card and its ID. */
		flip: string;
		now: number;
		/** Shows Resume and Remove buttons when given. */
		onRun?: (action: Action) => void;
	}

	let { job, flip, now, onRun }: Props = $props();
	const { session } = useApp();
	let name = $derived(displayName(job));
</script>

<!-- One download in a dashboard list: icon, name, what's going on. -->
<li class={job.state} data-flip={flip} out:leave>
	<span class="kind"><FileIcon {name} /></span>
	<span class="text">
		<span class="name" title={job.url} dir="auto">{name}</span>
		<span class="line">{describe(job, 0, now, undefined, session.slotNames)}</span>
	</span>
	{#if onRun}
		<span class="actions">
			{#if canResume(job)}
				<button
					onclick={() => onRun('resume')}
					aria-label={m.resume_label({ name })}
					title={m.resume()}
				>
					<Play size={15} />
				</button>
			{/if}
			<button
				onclick={() => onRun('remove')}
				aria-label={m.remove_label({ name })}
				title={m.remove()}
			>
				<Trash size={15} />
			</button>
		</span>
	{/if}
</li>

<style>
	/* A row like the dashboard's others: the file's kind where they show
	   the percent or the place. */
	li {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		min-height: 3.1rem;
		padding: 0.35rem 0.4rem 0.35rem 0.5rem;
		border-radius: var(--radius-m);
		background: var(--well);
	}

	.kind {
		display: grid;
		flex: none;
		place-items: center;
		width: 2.6rem;
		height: 2.3rem;
		border-radius: var(--radius-s);
		background: var(--hover);
		color: var(--muted);
	}

	.failed .kind {
		background: var(--danger-bg);
		color: var(--danger);
	}

	.retrying .kind {
		background: var(--warn-bg);
		color: var(--warn);
	}

	.completed .kind {
		background: var(--ok-bg);
		color: var(--ok);
	}

	.text {
		display: grid;
		flex: 1;
		min-width: 0;
	}

	.name,
	.line {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.name {
		font-weight: 500;
	}

	.line {
		color: var(--muted);
		font-size: 0.85em;
	}

	.failed .line {
		color: var(--danger);
	}

	.actions {
		display: flex;
		gap: 0.15rem;
	}

	button {
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-color: transparent;
		border-radius: 50%;
		background: none;
		color: var(--muted);
	}

	button:hover {
		background: var(--hover);
		color: var(--text);
	}
</style>
