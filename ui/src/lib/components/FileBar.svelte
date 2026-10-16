<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import type { Job } from '@ottie/api-types/Job';
	import { strip } from '#lib/filemap.js';
	import { fraction } from '#lib/status.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		job: Job;
		/** A row's thin bar, or the details panel's (docs/decisions/0027). */
		size: 'row' | 'detail';
	}

	let { job, size, ...attributes }: Props = $props();
	let done = $derived(fraction(job));
	// Each range at its place in the file, once the size is known.
	let parts = $derived(strip(job, job.state === 'downloading'));

	function percent(fraction: number): string {
		return `${fraction * 100}%`;
	}
</script>

<!-- The file from start to end: what has arrived, each connection's part at
     its place, and where each is writing now. The same in a row of the list
     and, bigger, in the details. The parent gives its role and label. -->
<div
	class="bar {size} {job.state}"
	class:unknown={done === null && job.state === 'downloading'}
	{...attributes}
>
	{#if parts}
		{#each parts as part (part.start)}
			<span
				class="range"
				class:writing={part.writing}
				style:inset-inline-start={percent(part.start)}
				style:width={percent(part.end - part.start)}
			>
				<span
					class="fill"
					style:width={percent(
						part.end > part.start ? (part.reached - part.start) / (part.end - part.start) : 1
					)}
				></span>
			</span>
		{/each}
	{:else}
		<span class="range" style:width="100%">
			<span class="fill" style:width={percent(done ?? 0)}></span>
		</span>
	{/if}
</div>

<style>
	/* One bar rounded at both ends, straight inside, with no hairlines
	   (docs/decisions/0044); where a connection writes glows. */
	.bar {
		position: relative;
		overflow: hidden;
		border-radius: 999px;
		background: var(--track);
	}

	.row {
		flex: 1;
		height: 0.5rem;
	}

	.detail {
		height: 0.85rem;
	}

	.range {
		position: absolute;
		top: 0;
		bottom: 0;
		overflow: hidden;
	}

	.fill {
		position: relative;
		display: block;
		height: 100%;
		background: var(--progress);
		transition: width 0.25s linear;
	}

	/* Where a connection is writing right now. */
	.writing .fill::after {
		position: absolute;
		top: 0;
		inset-inline-end: 0;
		bottom: 0;
		width: 2px;
		background: var(--arriving);
		content: '';
		box-shadow: 0 0 8px var(--glow);
	}

	.completed .fill {
		background: var(--ok);
	}

	.paused .fill,
	.queued .fill,
	.scheduled .fill,
	.cancelled .fill {
		background: var(--quiet);
	}

	.retrying .fill {
		background: var(--warn);
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
			margin-inline-start: 0;
		}

		to {
			margin-inline-start: 70%;
		}
	}

	.detail .writing .fill::after {
		width: 3px;
	}
</style>
