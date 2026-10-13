<script lang="ts">
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Link from '@lucide/svelte/icons/link';
	import Plus from '@lucide/svelte/icons/plus';
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
			showFolder = false;
		}
	}

	/** Closes the folder panel once focus leaves the add bar. */
	function focusOut(event: FocusEvent & { currentTarget: HTMLFormElement }) {
		const next = event.relatedTarget;
		if (!(next instanceof Node && event.currentTarget.contains(next))) {
			showFolder = false;
		}
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			showFolder = false;
		}
	}
</script>

<form onsubmit={submit} onfocusout={focusOut}>
	<label class="field">
		<Link size={15} aria-hidden="true" />
		<input
			bind:value={url}
			type="url"
			placeholder={m.add_placeholder()}
			aria-label={m.add_label()}
			required
		/>
	</label>
	<button
		type="button"
		class="icon"
		class:set={folder.trim() !== ''}
		aria-label={m.add_choose_folder()}
		aria-expanded={showFolder}
		title={folder.trim() || m.add_choose_folder()}
		onclick={() => (showFolder = !showFolder)}
	>
		<FolderOpen size={16} />
	</button>
	<button class="primary" disabled={adding || !url.trim()} aria-label={m.add_button()}>
		<Plus size={16} aria-hidden="true" />
		<span>{m.add_button()}</span>
	</button>

	{#if showFolder}
		<div class="folder">
			<!-- svelte-ignore a11y_autofocus -->
			<input
				bind:value={folder}
				placeholder={m.add_folder_placeholder()}
				aria-label={m.add_folder_label()}
				onkeydown={keydown}
				autofocus
			/>
		</div>
	{/if}
</form>

<style>
	form {
		position: relative;
		display: flex;
		gap: 0.4rem;
		width: 100%;
		max-width: 46rem;
	}

	.field {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
		height: 2.1rem;
		padding: 0 0.7rem;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface);
		color: var(--muted);
	}

	.field:focus-within {
		border-color: var(--accent);
	}

	.field input {
		flex: 1;
		min-width: 0;
		padding: 0;
		border: none;
		background: none;
		color: var(--text);
		outline: none;
	}

	button {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		height: 2.1rem;
		white-space: nowrap;
	}

	.icon {
		justify-content: center;
		width: 2.1rem;
		padding: 0;
		color: var(--muted);
	}

	.icon.set {
		color: var(--accent);
		border-color: var(--accent);
	}

	.folder {
		position: absolute;
		top: calc(100% + 0.4rem);
		right: 0;
		left: 0;
		z-index: 10;
		padding: 0.5rem;
		border: 1px solid var(--border);
		border-radius: 10px;
		background: var(--surface);
		box-shadow: var(--shadow);
	}

	.folder input {
		width: 100%;
	}

	/* Narrow: the button keeps only its icon. */
	@media (max-width: 30rem) {
		.primary span {
			display: none;
		}
	}
</style>
