<script lang="ts">
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
			placeholder="Paste a link to download"
			aria-label="Link to download"
			required
		/>
		<button class="primary" disabled={adding || !url.trim()}>Download</button>
	</div>
	<div class="row options">
		{#if showFolder}
			<input
				bind:value={folder}
				placeholder="Save in this folder (full path), instead of Downloads"
				aria-label="Folder"
			/>
		{:else}
			<button type="button" class="link" onclick={() => (showFolder = true)}>
				Choose a folder…
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
