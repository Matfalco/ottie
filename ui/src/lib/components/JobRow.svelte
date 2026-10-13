<script lang="ts">
	import type { Component } from 'svelte';
	import type { Job } from '@ottie/api-types/Job';
	import Disc from '@lucide/svelte/icons/disc-3';
	import File from '@lucide/svelte/icons/file';
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import FileImage from '@lucide/svelte/icons/file-image';
	import FileMusic from '@lucide/svelte/icons/file-music';
	import FileText from '@lucide/svelte/icons/file-text';
	import FileVideo from '@lucide/svelte/icons/file-video-camera';
	import Pause from '@lucide/svelte/icons/pause';
	import Play from '@lucide/svelte/icons/play';
	import Trash from '@lucide/svelte/icons/trash';
	import type { Action } from '#lib/bulk.js';
	import { fileKind, type FileKind } from '#lib/filetypes.js';
	import { displayName, formatAdded, formatBytes, formatPercent } from '#lib/format.js';
	import { addedAt } from '#lib/jobs.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { Keys } from '#lib/selection.js';
	import { canPause, canResume, describe, fraction, motion } from '#lib/status.js';

	interface Props {
		job: Job;
		selected: boolean;
		/** Bytes per second right now; 0 if unknown. */
		speed: number;
		now: number;
		onClick: (keys: Keys) => void;
		onRun: (action: Action) => void;
	}

	let { job, selected, speed, now, onClick, onRun }: Props = $props();

	const ICONS: Record<FileKind, Component> = {
		video: FileVideo,
		audio: FileMusic,
		image: FileImage,
		archive: FileArchive,
		document: FileText,
		program: Disc,
		other: File
	};

	let name = $derived(displayName(job));
	let Icon = $derived(ICONS[fileKind(name)]);
	let done = $derived(fraction(job));
	let moving = $derived(motion(job, speed, now));
	let added = $derived(addedAt(job.id));
	let size = $derived(
		job.total !== null
			? formatBytes(job.total)
			: job.state === 'completed'
				? formatBytes(job.downloaded)
				: '—'
	);

	function click(event: MouseEvent) {
		onClick({ toggle: event.ctrlKey || event.metaKey, range: event.shiftKey });
	}

	/** The row's own buttons and checkbox don't change the selection like a row click. */
	function run(event: MouseEvent, action: Action) {
		event.stopPropagation();
		onRun(action);
	}
</script>

<!-- Clicking the row selects it, as in a file manager; from the keyboard the
     row's checkbox does the same. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<tr class={job.state} class:selected onclick={click}>
	<td class="check">
		<input
			type="checkbox"
			checked={selected}
			aria-label={m.select_row({ name })}
			onclick={(event) => {
				event.stopPropagation();
				onClick({ toggle: true, range: event.shiftKey });
			}}
		/>
	</td>
	<td class="name" title={job.url}>
		<div class="cell">
			<span class="kind"><Icon size={16} aria-hidden="true" /></span>
			<span class="text">
				<span class="title">{name}</span>
				<span class="line">{describe(job, speed, now)}</span>
			</span>
		</div>
	</td>
	<td class="size">{size}</td>
	<td class="progress">
		<div class="cell">
			<div
				class="bar"
				class:unknown={done === null && job.state === 'downloading'}
				role="progressbar"
				aria-label={m.progress_label({ name })}
				aria-valuemin={0}
				aria-valuemax={100}
				aria-valuenow={done === null ? undefined : Math.floor(done * 100)}
			>
				<div class="fill" style:width={`${(done ?? 0) * 100}%`}></div>
			</div>
			<span class="percent">{done === null ? '' : formatPercent(done)}</span>
		</div>
	</td>
	{#if 'status' in moving}
		<td class="status" colspan="2" title={moving.status}>{moving.status}</td>
	{:else}
		<td class="speed">{moving.speed}</td>
		<td class="left">{moving.left}</td>
	{/if}
	<td class="added">{added ? formatAdded(added, new Date(now)) : ''}</td>
	<td class="actions">
		{#if canPause(job)}
			<button
				onclick={(event) => run(event, 'pause')}
				aria-label={m.pause_label({ name })}
				title={m.pause()}
			>
				<Pause size={15} />
			</button>
		{:else if canResume(job)}
			<button
				onclick={(event) => run(event, 'resume')}
				aria-label={m.resume_label({ name })}
				title={m.resume()}
			>
				<Play size={15} />
			</button>
		{/if}
		<button
			onclick={(event) => run(event, 'remove')}
			aria-label={m.remove_label({ name })}
			title={m.remove()}
		>
			<Trash size={15} />
		</button>
	</td>
</tr>

<style>
	tr {
		height: 2.6rem;
		cursor: default;
		user-select: none;
	}

	tr:hover {
		background: var(--hover);
	}

	tr.selected {
		background: var(--selected);
	}

	td {
		padding: 0 0.5rem;
		border-bottom: 1px solid var(--border-soft);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}

	.check {
		padding-right: 0;
		text-align: center;
	}

	input[type='checkbox'] {
		margin: 0;
		vertical-align: middle;
	}

	.name .cell {
		display: flex;
		align-items: center;
		gap: 0.55rem;
	}

	.kind {
		display: grid;
		flex: none;
		color: var(--muted);
	}

	.text {
		display: grid;
		min-width: 0;
	}

	.title {
		overflow: hidden;
		text-overflow: ellipsis;
		font-weight: 500;
	}

	/* The full description, only on phones (the columns say it otherwise). */
	.line {
		display: none;
	}

	.size,
	.speed,
	.left,
	.added {
		color: var(--muted);
		text-align: right;
	}

	.status {
		color: var(--muted);
	}

	.failed .status {
		color: var(--danger);
	}

	.progress .cell {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.bar {
		flex: 1;
		height: 5px;
		border-radius: 3px;
		background: var(--track);
		overflow: hidden;
	}

	.fill {
		height: 100%;
		border-radius: 3px;
		background: var(--progress);
		transition: width 0.25s linear;
	}

	.completed .fill {
		background: var(--accent);
	}

	.paused .fill,
	.cancelled .fill,
	.queued .fill {
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

	.percent {
		width: 2.6rem;
		color: var(--muted);
		font-size: 0.85em;
		text-align: right;
	}

	.actions {
		padding: 0 0.25rem;
		text-align: right;
	}

	.actions button {
		display: inline-grid;
		place-items: center;
		width: 1.9rem;
		height: 1.9rem;
		padding: 0;
		border-color: transparent;
		background: none;
		color: var(--muted);
		/* Shown on hover, focus or selection, so the list stays calm. */
		opacity: 0;
	}

	tr:hover .actions button,
	tr.selected .actions button,
	.actions button:focus-visible {
		opacity: 1;
	}

	.actions button:hover {
		background: var(--surface);
		color: var(--text);
	}

	/* Touch screens have no hover: always show them. */
	@media (hover: none) {
		.actions button {
			opacity: 1;
		}
	}

	@media (max-width: 64rem) {
		.added {
			display: none;
		}
	}

	@media (max-width: 52rem) {
		.size {
			display: none;
		}
	}

	/* Phones: name and description, then the bar; no columns. */
	@media (max-width: 40rem) {
		tr {
			display: grid;
			grid-template-columns: 2rem minmax(0, 1fr) auto;
			grid-template-areas: 'check name actions' '. progress progress';
			height: auto;
			padding: 0.5rem 0.25rem 0.6rem;
			border-bottom: 1px solid var(--border-soft);
		}

		td {
			border-bottom: none;
		}

		.check {
			grid-area: check;
			align-self: center;
		}

		.name {
			grid-area: name;
		}

		.line {
			display: block;
			overflow: hidden;
			text-overflow: ellipsis;
			color: var(--muted);
			font-size: 0.85em;
		}

		.failed .line {
			color: var(--danger);
		}

		.actions {
			grid-area: actions;
			align-self: center;
		}

		.progress {
			grid-area: progress;
			padding-top: 0.35rem;
		}

		.percent,
		.status,
		.speed,
		.left {
			display: none;
		}
	}
</style>
