<script lang="ts">
	import type { Job } from '@ottie/api-types/Job';
	import { displayName } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		canCancel,
		canPause,
		canResume,
		describe,
		fraction,
		removeLosesData
	} from '#lib/status.js';

	interface Props {
		job: Job;
		/** Bytes per second right now; 0 if unknown. */
		speed: number;
		now: number;
		onPause: () => void;
		onResume: () => void;
		onCancel: () => void;
		onRemove: () => void;
	}

	let { job, speed, now, onPause, onResume, onCancel, onRemove }: Props = $props();
	let name = $derived(displayName(job));
	let done = $derived(fraction(job));

	function confirmCancel() {
		if (confirm(m.cancel_confirm({ name }))) {
			onCancel();
		}
	}

	// A completed file stays on disk, so there's nothing to confirm then.
	function confirmRemove() {
		if (!removeLosesData(job) || confirm(m.remove_confirm({ name }))) {
			onRemove();
		}
	}
</script>

<li class={job.state}>
	<div class="text">
		<span class="name" title={job.url}>{name}</span>
		<span class="status">{describe(job, speed, now)}</span>
	</div>
	<div class="actions">
		{#if canPause(job)}
			<button onclick={onPause}>{m.pause()}</button>
		{/if}
		{#if canResume(job)}
			<button onclick={onResume}>{m.resume()}</button>
		{/if}
		{#if canCancel(job)}
			<button onclick={confirmCancel} aria-label={m.cancel_label({ name })}>{m.cancel()}</button>
		{/if}
		<button onclick={confirmRemove} aria-label={m.remove_label({ name })}>{m.remove()}</button>
	</div>
	<div
		class="bar"
		class:unknown={done === null && job.state === 'downloading'}
		role="progressbar"
		aria-label={m.progress_label({ name })}
		aria-valuemin={0}
		aria-valuemax={100}
		aria-valuenow={done === null ? undefined : Math.round(done * 100)}
	>
		<div class="fill" style:width={`${(done ?? 0) * 100}%`}></div>
	</div>
</li>

<style>
	li {
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 0.35rem 1rem;
		align-items: center;
		padding: 0.8rem 1rem;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 12px;
	}

	.text {
		display: grid;
		min-width: 0;
	}

	.name {
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status {
		color: var(--muted);
		font-size: 0.9em;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.failed .status {
		color: var(--danger);
	}

	.actions {
		display: flex;
		gap: 0.4rem;
	}

	/* Phones: the buttons get a row of their own, so the name keeps its room. */
	@media (max-width: 40rem) {
		.actions {
			grid-column: 1 / -1;
			order: 1;
		}
	}

	.bar {
		grid-column: 1 / -1;
		height: 6px;
		border-radius: 3px;
		background: var(--track);
		overflow: hidden;
	}

	.fill {
		height: 100%;
		background: var(--progress);
		transition: width 0.25s linear;
	}

	.completed .fill {
		background: var(--accent);
	}

	.paused .fill,
	.cancelled .fill {
		background: var(--muted);
	}

	.failed .fill {
		background: var(--danger);
	}

	/* Size unknown: a moving stripe instead of a percentage. */
	.unknown .fill {
		width: 30% !important;
		animation: slide 1.2s ease-in-out infinite alternate;
	}

	@keyframes slide {
		from {
			margin-left: 0;
		}
		to {
			margin-left: 70%;
		}
	}
</style>
