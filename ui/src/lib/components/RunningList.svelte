<script lang="ts">
	import type { Job } from '@ottie/api-types/Job';
	import { displayName } from '#lib/format.js';
	import { currentSpeed, type Rate } from '#lib/jobs.js';
	import { m } from '#lib/paraglide/messages.js';
	import { describe, fraction, motion } from '#lib/status.js';

	interface Props {
		/** Downloading now. */
		running: Job[];
		rates: Record<string, Rate>;
		now: number;
		/** The time slots' names by ID. */
		slots: Record<number, string>;
	}

	let { running, rates, now, slots }: Props = $props();
</script>

<!-- The tray menu's running downloads, each with its bar. It's outside the
     shell, so it takes everything as props: useApp() would throw there. -->
{#if running.length > 0}
	<ul class="running">
		{#each running as job (job.id)}
			{@const speed = currentSpeed(rates[job.id], now)}
			{@const moving = motion(job, speed, now, undefined, slots)}
			{@const done = fraction(job)}
			<li>
				<span class="top">
					<span class="name" title={job.url} dir="auto">{displayName(job)}</span>
					<span class="speed">{'speed' in moving ? moving.speed : ''}</span>
				</span>
				<span class="bar" class:unknown={done === null}>
					<span class="fill" style:width={`${(done ?? 0) * 100}%`}></span>
				</span>
				<span class="line">{describe(job, speed, now, undefined, slots)}</span>
			</li>
		{/each}
	</ul>
{:else}
	<p class="none">{m.dash_running_none()}</p>
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
		background: var(--track);
		border-radius: 999px;
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
			margin-inline-start: 0;
		}

		to {
			margin-inline-start: 70%;
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
</style>
