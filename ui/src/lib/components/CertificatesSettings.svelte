<script lang="ts">
	// Settings → Connection: FTPS servers' certificates trusted by hand, from
	// a download that stopped at one (docs/decisions/0054). Shown once there
	// is one; removing it refuses that certificate again.
	import type { TrustedCertificate } from '@ottie/api-types/TrustedCertificate';
	import X from '@lucide/svelte/icons/x';
	import { onMount } from 'svelte';
	import { formatFingerprint } from '#lib/certificates.js';
	import { useApp } from '#lib/context.js';
	import { leave, shuffle } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	const { session } = useApp();

	let certificates: TrustedCertificate[] = $state([]);
	let problem: string | null = $state(null);
	let list: HTMLUListElement | undefined = $state();

	// Rows slide into place when one goes (docs/decisions/0048).
	shuffle(
		() => list,
		() => certificates.map((certificate) => certificate.id)
	);

	onMount(async () => {
		const answer = await session.certificates();
		if (answer.ok) {
			certificates = answer.value;
		} else {
			problem = answer.why;
		}
	});

	async function remove(id: number) {
		if (await session.distrustCertificate(id)) {
			certificates = certificates.filter((certificate) => certificate.id !== id);
		}
	}
</script>

{#if certificates.length > 0 || problem}
	<div class="block">
		<span class="names">
			<span>{m.cert_title()}</span>
			<span class="hint">{m.cert_hint()}</span>
		</span>
		<ul class="rows" bind:this={list}>
			{#each certificates as certificate (certificate.id)}
				<li data-flip={certificate.id} out:leave>
					<span class="names">
						<span dir="auto">{certificate.host}</span>
						<span class="hint fingerprint" dir="ltr">
							{formatFingerprint(certificate.fingerprint)}
						</span>
					</span>
					{#if !session.readOnly}
						<button
							class="round"
							onclick={() => void remove(certificate.id)}
							aria-label={m.cert_remove_label({ host: certificate.host })}
							title={m.cert_remove_label({ host: certificate.host })}
						>
							<X size={15} aria-hidden="true" />
						</button>
					{/if}
				</li>
			{/each}
		</ul>
		{#if problem}
			<p class="problem" role="alert">{problem}</p>
		{/if}
	</div>
{/if}

<style>
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 0.8rem 1.25rem;
		border-top: 1px solid var(--border-soft);
	}

	.names {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}

	.hint {
		color: var(--muted);
		font-size: 0.85rem;
	}

	.rows {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.rows li {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.45rem 0.5rem 0.45rem 0.85rem;
		border-radius: var(--radius-m);
		background: var(--sunken);
	}

	.rows .names {
		flex: 1;
	}

	/* 95 characters: it wraps rather than hides, so it can be compared. */
	.fingerprint {
		font-family: var(--mono);
		font-size: 0.75rem;
		overflow-wrap: anywhere;
		text-align: start;
	}

	.round {
		display: grid;
		flex: none;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-radius: var(--radius-pill);
	}

	.problem {
		margin: 0;
		color: var(--danger);
	}
</style>
