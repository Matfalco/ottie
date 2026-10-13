<script lang="ts">
	import type { Job } from '@ottie/api-types/Job';
	import { displayName } from '#lib/format.js';
	import { currentSpeed, type Rate } from '#lib/jobs.js';
	import { m } from '#lib/paraglide/messages.js';
	import { describe, fraction, motion } from '#lib/status.js';

	interface Props {
		/** Downloading now. */
		running: Job[];
		/** Waiting for a slot, first in line first. */
		next: Job[];
		rates: Record<string, Rate>;
		now: number;
	}

	let { running, next, rates, now }: Props = $props();
</script>

<!-- The dashboard's "Downloading now" card: each running download with its
     bar, then what's next in line. -->
{#if running.length > 0}
	<ul class="running">
		{#each running as job (job.id)}
			{@const speed = currentSpeed(rates[job.id], now)}
			{@const moving = motion(job, speed, now)}
			{@const done = fraction(job)}
			<li>
				<span class="top">
					<span class="name" title={job.url}>{displayName(job)}</span>
					<span class="speed">{'speed' in moving ? moving.speed : ''}</span>
				</span>
				<span class="bar" class:unknown={done === null}>
					<span class="fill" style:width={`${(done ?? 0) * 100}%`}></span>
				</span>
				<span class="line">{describe(job, speed, now)}</span>
			</li>
		{/each}
	</ul>
{:else}
	<p class="none">{m.dash_running_none()}</p>
{/if}

{#if next.length > 0}
	<div class="next">
		<h3>{m.dash_next()}</h3>
		<ul>
			{#each next as job (job.id)}
				<li>
					<span class="name" title={job.url}>{displayName(job)}</span>
				</li>
			{/each}
		</ul>
	</div>
{/if}

<style>
	ul {
		display: grid;
		gap: 0.9rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.running li {
		display: grid;
		gap: 0.35rem;
	}

	.top {
		display: flex;
		align-items: baseline;
		gap: 0.75rem;
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.speed {
		color: var(--muted);
		font-family: var(--mono);
		font-size: 0.85em;
	}

	.bar {
		height: 6px;
		overflow: hidden;
		border-radius: 3px;
		background: var(--track);
	}

	.fill {
		display: block;
		height: 100%;
		background: var(--progress);
		transition: width 0.25s linear;
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

	.line {
		overflow: hidden;
		color: var(--faint);
		font-size: 0.85em;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.none {
		margin: 0.5rem 0;
		color: var(--muted);
	}

	.next {
		display: grid;
		gap: 0.4rem;
		margin-top: 1rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border-soft);
	}

	h3 {
		margin: 0;
		color: var(--faint);
		font-size: 0.85em;
		font-weight: 500;
	}

	.next ul {
		gap: 0.3rem;
	}

	.next .name {
		display: block;
		font-weight: 400;
	}
</style>
