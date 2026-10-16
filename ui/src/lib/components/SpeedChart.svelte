<script lang="ts">
	import { formatDuration, formatSpeed } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { POINTS, niceTop, overSpan, paths, points, pointsAgo, type Span } from '#lib/speed.js';

	interface Props {
		/** One total-speed sample per second, oldest first. */
		history: number[];
		/** How far back it shows, in seconds. */
		span: Span;
	}

	let { history, span }: Props = $props();

	// Drawn in a fixed box and stretched to the card's width; strokes keep
	// their width (`vector-effect`), labels are HTML so they don't stretch.
	const WIDTH = 600;
	const HEIGHT = 120;

	/** A long span averages neighbouring seconds: `step` of them per point. */
	let shown = $derived(overSpan(history, span));
	let samples = $derived(shown.samples);
	let peak = $derived(Math.max(0, ...samples));
	let top = $derived(niceTop(peak));
	let at = $derived(points(samples, WIDTH, HEIGHT, top, POINTS));
	let shape = $derived(paths(at, HEIGHT));
	let summary = $derived(
		m.dash_speed_over({
			time: formatDuration(span),
			now: formatSpeed(samples.at(-1) ?? 0),
			peak: formatSpeed(peak)
		})
	);

	/** The point under the pointer, as points before the newest. */
	let hovered: number | null = $state(null);
	let hoveredIndex = $derived(hovered === null ? -1 : samples.length - 1 - hovered);
	let hoveredPoint = $derived(hoveredIndex >= 0 ? at[hoveredIndex] : undefined);

	function move(event: PointerEvent & { currentTarget: HTMLElement }) {
		const box = event.currentTarget.getBoundingClientRect();
		const rtl = getComputedStyle(event.currentTarget).direction === 'rtl';
		hovered = pointsAgo(event.clientX, box.left, box.width, rtl, POINTS);
	}

	function ago(seconds: number): string {
		return seconds === 0 ? m.dash_now() : m.dash_ago({ time: formatDuration(seconds) });
	}
</script>

<figure>
	<div
		class="plot"
		role="img"
		aria-label={summary}
		onpointermove={move}
		onpointerleave={() => (hovered = null)}
	>
		<svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" aria-hidden="true">
			<defs>
				<!-- The rounded style's fill fades toward the bottom. -->
				<linearGradient id="speed-fill" x1="0" y1="0" x2="0" y2="1">
					<stop offset="0" class="fade-top" />
					<stop offset="1" class="fade-bottom" />
				</linearGradient>
			</defs>
			<line class="grid" x1="0" x2={WIDTH} y1="0.5" y2="0.5" />
			<line class="grid" x1="0" x2={WIDTH} y1={HEIGHT / 2} y2={HEIGHT / 2} />
			<line class="base" x1="0" x2={WIDTH} y1={HEIGHT - 0.5} y2={HEIGHT - 0.5} />
			<path class="area" d={shape.area} />
			<path class="line" d={shape.line} />
			{#if hoveredPoint}
				<line class="crosshair" x1={hoveredPoint.x} x2={hoveredPoint.x} y1="0" y2={HEIGHT} />
			{/if}
		</svg>
		<span class="tick" style:top="1px">{formatSpeed(top)}</span>
		<span class="tick" style:top="50%">{formatSpeed(top / 2)}</span>
		{#if hoveredPoint && hovered !== null}
			<!-- Flips before the crosshair in the half toward now, so it stays inside. -->
			<div
				class="tooltip"
				class:before={hoveredPoint.x > WIDTH / 2}
				style:inset-inline-start={`${(hoveredPoint.x / WIDTH) * 100}%`}
			>
				<strong>{formatSpeed(samples[hoveredIndex])}</strong>
				<span>{ago(hovered * shown.step)}</span>
			</div>
		{/if}
	</div>
	<figcaption>
		<span>{ago(span)}</span>
		<span class="now">{m.dash_now()}</span>
	</figcaption>
</figure>

<style>
	figure {
		margin: 0;
	}

	.plot {
		position: relative;
		height: 11rem;
		touch-action: pan-y;
	}

	/* Time runs with the text: now at the end of the line, so on the left
	   right to left (docs/decisions/0042). */
	svg:dir(rtl) {
		transform: scaleX(-1);
	}

	svg {
		display: block;
		width: 100%;
		height: 100%;
		overflow: visible;
	}

	line,
	path {
		vector-effect: non-scaling-stroke;
	}

	.grid {
		stroke: var(--border-soft);
		stroke-width: 1;
	}

	.base {
		stroke: var(--border);
		stroke-width: 1;
	}

	.area {
		fill: var(--progress);
		opacity: 0.1;
	}

	.line {
		fill: none;
		stroke: var(--progress);
		stroke-width: 2;
		stroke-linejoin: round;
		stroke-linecap: round;
	}

	.fade-top {
		stop-color: var(--progress);
		stop-opacity: 0.4;
	}

	.fade-bottom {
		stop-color: var(--progress);
		stop-opacity: 0;
	}

	.crosshair {
		stroke: var(--muted);
		stroke-width: 1;
	}

	/* Just above its gridline, so it never covers the line. */
	.tick {
		position: absolute;
		inset-inline-end: 0;
		color: var(--muted);
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
		line-height: 1;
		transform: translateY(calc(-100% - 3px));
	}

	.tooltip {
		position: absolute;
		top: 0.25rem;
		display: grid;
		margin-inline-start: 0.5rem;
		padding: 0.35rem 0.55rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--surface);
		box-shadow: var(--shadow);
		font-size: 0.85rem;
		white-space: nowrap;
		pointer-events: none;
	}

	.tooltip.before {
		margin-inline-start: 0;
		transform: translateX(calc(-100% - 0.5rem));
	}

	.tooltip.before:dir(rtl) {
		transform: translateX(calc(100% + 0.5rem));
	}

	.tooltip span {
		color: var(--muted);
	}

	figcaption {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 0.3rem;
		color: var(--muted);
		font-size: 0.75rem;
	}

	/* Rounded (docs/decisions/0044): a thicker line over a fading fill, the
	   grid dotted, the tooltip and "now" as pills. */
	:global([data-style='rounded']) .area {
		fill: url(#speed-fill);
		opacity: 1;
	}

	:global([data-style='rounded']) .line {
		stroke-width: 2.6;
	}

	:global([data-style='rounded']) .grid {
		stroke-dasharray: 2 6;
		stroke-linecap: round;
	}

	:global([data-style='rounded']) .tooltip {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		padding: 0.3rem 0.75rem;
		border: none;
		border-radius: 999px;
		background: var(--text);
		color: var(--bg);
	}

	:global([data-style='rounded']) .tooltip span {
		color: var(--bg);
		opacity: 0.7;
	}

	:global([data-style='rounded']) .now {
		padding: 0.1rem 0.6rem;
		border-radius: 999px;
		background: var(--selected);
		color: var(--text);
	}
</style>
