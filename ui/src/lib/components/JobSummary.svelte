<script lang="ts">
	import type { Job } from '@ottie/api-types/Job';
	import Play from '@lucide/svelte/icons/play';
	import Trash from '@lucide/svelte/icons/trash';
	import type { Action } from '#lib/bulk.js';
	import { displayName } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { canResume, describe } from '#lib/status.js';
	import FileIcon from './FileIcon.svelte';

	interface Props {
		job: Job;
		now: number;
		/** Shows Resume and Remove buttons when given. */
		onRun?: (action: Action) => void;
	}

	let { job, now, onRun }: Props = $props();
	let name = $derived(displayName(job));
</script>

<!-- One download in a dashboard list: icon, name, what's going on. -->
<li class={job.state}>
	<span class="kind"><FileIcon {name} /></span>
	<span class="text">
		<span class="name" title={job.url}>{name}</span>
		<span class="line">{describe(job, 0, now)}</span>
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
	li {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		min-height: 2.75rem;
		padding: 0.35rem 0;
		border-bottom: 1px solid var(--border-soft);
	}

	li:last-child {
		border-bottom: none;
	}

	.kind {
		display: grid;
		color: var(--muted);
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
		width: 1.9rem;
		height: 1.9rem;
		padding: 0;
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	button:hover {
		background: var(--hover);
		color: var(--text);
	}
</style>
