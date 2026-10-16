<script lang="ts">
	import type { Component } from 'svelte';
	import type { Job } from '@ottie/api-types/Job';
	import CalendarClock from '@lucide/svelte/icons/calendar-clock';
	import Download from '@lucide/svelte/icons/download';
	import LayoutDashboard from '@lucide/svelte/icons/layout-dashboard';
	import Settings from '@lucide/svelte/icons/settings';
	import Tags from '@lucide/svelte/icons/tags';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { isActive } from '#lib/filters.js';
	import { formatCount } from '#lib/format.js';
	import { shuffle } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		jobs: Job[];
		/** Icons only, their names in tooltips: the title bar is short of room. */
		compact?: boolean;
	}

	let { jobs, compact = false }: Props = $props();
	let active = $derived(jobs.filter(isActive).length);
	let tabs: HTMLUListElement | undefined = $state();
	// The current tab's highlight slides to the next one (docs/decisions/0048).
	shuffle(
		() => tabs,
		() => page.url.pathname
	);

	interface Entry {
		href: string;
		label: () => string;
		icon: Component;
		/** A number beside the name, when there's something to count. */
		badge?: () => number;
	}

	const ENTRIES: Entry[] = [
		{ href: resolve('/'), label: m.nav_dashboard, icon: LayoutDashboard },
		{ href: resolve('/downloads'), label: m.nav_downloads, icon: Download, badge: () => active },
		{ href: resolve('/categories'), label: m.nav_categories, icon: Tags },
		{ href: resolve('/schedule'), label: m.nav_schedule, icon: CalendarClock },
		{ href: resolve('/settings'), label: m.nav_settings, icon: Settings }
	];
</script>

<!-- The sections as tabs in the title bar; a bar along the bottom on phones. -->
<nav aria-label={m.nav_label()} class:compact>
	<ul bind:this={tabs}>
		{#each ENTRIES as { href, label, icon: Icon, badge } (href)}
			{@const current = page.url.pathname === href}
			{@const count = badge?.() ?? 0}
			<li>
				<a {href} class:current aria-current={current ? 'page' : undefined} title={label()}>
					{#if current}
						<span class="highlight" data-flip="highlight" data-flip-size></span>
					{/if}
					<span class="icon"><Icon size={18} aria-hidden="true" /></span>
					<span class="name">{label()}</span>
					{#if count > 0}
						<span class="badge">{formatCount(count)}</span>
					{/if}
				</a>
			</li>
		{/each}
	</ul>
</nav>

<style>
	/* The tabs sit in a sunken track, the current one raised (its highlight,
	   which slides from tab to tab: docs/decisions/0048). */
	ul {
		display: flex;
		gap: 0.15rem;
		margin: 0;
		padding: 0.25rem;
		border-radius: 999px;
		background: var(--sunken);
		box-shadow: var(--sunken-shadow);
		list-style: none;
	}

	a {
		position: relative;
		/* The highlight goes under the tab's content but over its hover. */
		isolation: isolate;
		display: flex;
		align-items: center;
		gap: 0.45rem;
		height: 2.1rem;
		padding: 0 0.95rem;
		border-radius: 999px;
		color: var(--muted);
		text-decoration: none;
		white-space: nowrap;
	}

	a:hover {
		background: var(--hover);
		color: var(--text);
	}

	a.current {
		color: var(--text);
		font-weight: 600;
	}

	.icon {
		display: grid;
	}

	.badge {
		min-width: 1.25rem;
		padding: 0 0.35rem;
		border-radius: 999px;
		background: var(--accent);
		color: var(--on-accent);
		font-size: 0.72rem;
		font-weight: 600;
		line-height: 1.15rem;
		text-align: center;
	}

	/* Short of room in the title bar: icons only. */
	@media (min-width: 40.01rem) {
		.compact a {
			padding: 0 0.6rem;
		}

		.compact .name {
			display: none;
		}

		.compact .badge {
			position: absolute;
			top: -0.3rem;
			inset-inline-end: -0.35rem;
			min-width: 1.05rem;
			padding: 0 0.25rem;
			font-size: 0.65rem;
			line-height: 1.05rem;
		}
	}

	/* Phones: a bar along the bottom, like an Android app. */
	@media (max-width: 40rem) {
		li {
			flex: 1;
		}

		nav {
			position: fixed;
			inset-inline-end: 0;
			bottom: 0;
			inset-inline-start: 0;
			z-index: 5;
			padding: 0.3rem 0.5rem;
			border-top: none;
			background: var(--frame);
		}

		ul {
			justify-content: space-around;
			padding: 0;
			background: none;
			box-shadow: none;
		}

		a {
			flex-direction: column;
			justify-content: center;
			gap: 0.1rem;
			font-size: 0.75rem;
			border-radius: var(--radius-m);
		}

		a.current {
			color: var(--accent);
		}

		/* Five names don't fit side by side: only the current tab's. */
		a:not(.current) .name {
			position: absolute;
			width: 1px;
			height: 1px;
			overflow: hidden;
			clip-path: inset(50%);
		}

		.badge {
			position: absolute;
			top: 0.15rem;
			inset-inline-start: calc(50% + 0.4rem);
			min-width: 1.1rem;
			font-size: 0.65rem;
			line-height: 1.1rem;
		}
	}
</style>
