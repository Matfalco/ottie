<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** Resolves to true once the daemon has accepted the download. */
		onAdd: (url: string, destDir?: string) => Promise<boolean>;
	}

	let { onAdd }: Props = $props();
	let url = $state('');
	let folder = $state('');
	let showFolder = $state(false);
	let adding = $state(false);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		adding = true;
		const added = await onAdd(url.trim(), folder.trim() || undefined);
		adding = false;
		if (added) {
			url = '';
		}
	}
</script>

<form onsubmit={submit}>
	<div class="row">
		<input
			bind:value={url}
			type="url"
			placeholder={m.add_placeholder()}
			aria-label={m.add_label()}
			required
		/>
		<button class="primary" disabled={adding || !url.trim()}>{m.add_button()}</button>
	</div>
	<div class="row options">
		{#if showFolder}
			<input
				bind:value={folder}
				placeholder={m.add_folder_placeholder()}
				aria-label={m.add_folder_label()}
			/>
		{:else}
			<button type="button" class="link" onclick={() => (showFolder = true)}>
				{m.add_choose_folder()}
			</button>
		{/if}
	</div>
</form>

<style>
	form {
		display: grid;
		gap: 0.4rem;
	}

	.row {
		display: flex;
		gap: 0.5rem;
	}

	input {
		flex: 1;
		min-width: 0;
	}

	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--muted);
		font-size: 0.9em;
	}

	.link:hover {
		color: var(--text);
		text-decoration: underline;
	}
</style>
