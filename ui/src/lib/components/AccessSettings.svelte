<script lang="ts">
	// Settings → Connection: the devices with tokens of their own and the
	// folders they may use (docs/decisions/0052). The computer running Ottie
	// manages them; a device sees what it is.
	import type { Device } from '@ottie/api-types/Device';
	import Copy from '@lucide/svelte/icons/copy';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import X from '@lucide/svelte/icons/x';
	import { untrack } from 'svelte';
	import { deviceLine, folderRows, withFolder } from '#lib/access.js';
	import { useApp } from '#lib/context.js';
	import { leave, shuffle, unfold } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { signInLink } from '#lib/token.js';
	import { canPickFolder, pickFolder } from '#lib/window.js';
	import Switch from './Switch.svelte';

	const { session, ask } = useApp();
	const access = $derived(session.access);

	let devices: Device[] = $state([]);
	let name = $state('');
	let readOnly = $state(false);
	/** The device just added and its sign-in link, shown this once. */
	let added: { name: string; link: string } | null = $state(null);
	let copied = $state(false);
	let folder = $state('');
	let problem: string | null = $state(null);
	let list: HTMLUListElement | undefined = $state();

	// Rows slide into place when one goes (docs/decisions/0048).
	shuffle(
		() => list,
		() => devices.map((device) => device.id)
	);

	$effect(() => {
		if (session.owner) {
			untrack(() => void load());
		}
	});

	async function load() {
		const answer = await session.devices();
		if (answer.ok) {
			devices = answer.value;
		} else {
			problem = answer.why;
		}
	}

	async function add(event: SubmitEvent) {
		event.preventDefault();
		const answer = await session.addDevice(name.trim(), readOnly);
		if (!answer.ok) {
			problem = answer.why;
			return;
		}
		problem = null;
		devices = [...devices, answer.value.device];
		added = {
			name: answer.value.device.name,
			link: signInLink(location.origin, answer.value.token)
		};
		copied = false;
		name = '';
		readOnly = false;
	}

	async function remove(device: Device) {
		const message = m.set_device_remove_confirm({ name: device.name });
		if (!(await ask(message, m.set_device_remove_yes()))) {
			return;
		}
		if (await session.removeDevice(device.id)) {
			devices = devices.filter((each) => each.id !== device.id);
			if (added?.name === device.name) {
				added = null;
			}
		}
	}

	async function copy(link: string) {
		try {
			await navigator.clipboard.writeText(link);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			// Not allowed here (a plain-http remote page): the link is there to copy.
		}
	}

	async function addFolder(event: SubmitEvent) {
		event.preventDefault();
		const chosen = access && withFolder(access.folders, folder);
		if (!chosen) {
			return;
		}
		problem = await session.setFolders(chosen);
		if (problem === null) {
			folder = '';
		}
	}

	async function pick() {
		const chosen = await pickFolder(folder);
		if (chosen !== null) {
			folder = chosen;
		}
	}

	async function removeFolder(gone: string) {
		if (access) {
			problem = await session.setFolders(access.folders.chosen.filter((each) => each !== gone));
		}
	}
</script>

{#if access?.device}
	<div class="setting">
		<span class="names">
			<span>{m.set_this_device({ name: access.device.name })}</span>
			<span class="hint">{deviceLine(access.device)}</span>
			<span class="hint">{m.set_owner_only_hint()}</span>
		</span>
	</div>
{:else if access}
	<div class="block">
		<span class="names">
			<span>{m.set_devices()}</span>
			<span class="hint">{m.set_devices_hint()}</span>
		</span>
		{#if devices.length > 0}
			<ul class="rows" bind:this={list}>
				{#each devices as device (device.id)}
					<li data-flip={device.id} out:leave>
						<span class="names">
							<span dir="auto">{device.name}</span>
							<span class="hint">{deviceLine(device)}</span>
						</span>
						<button
							class="round"
							onclick={() => void remove(device)}
							aria-label={m.set_device_remove_label({ name: device.name })}
							title={m.set_device_remove_label({ name: device.name })}
						>
							<X size={15} aria-hidden="true" />
						</button>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="hint none">{m.set_devices_none()}</p>
		{/if}
		<form class="add" onsubmit={add}>
			<input
				bind:value={name}
				maxlength="60"
				dir="auto"
				placeholder={m.set_device_name()}
				aria-label={m.set_device_name()}
			/>
			<span class="switch">
				<Switch on={readOnly} label={m.set_device_read_only()} onChange={(on) => (readOnly = on)} />
				<span aria-hidden="true">{m.set_device_read_only()}</span>
			</span>
			<button type="submit" disabled={name.trim() === ''}>{m.set_device_add()}</button>
		</form>
		{#if added}
			<div class="link" in:unfold out:unfold>
				<span>{m.set_device_link({ name: added.name })}</span>
				<span class="field">
					<input
						readonly
						value={added.link}
						dir="ltr"
						aria-label={m.set_link_label()}
						onfocus={(event) => event.currentTarget.select()}
					/>
					<button class="copy" onclick={() => added && void copy(added.link)}>
						<Copy size={15} aria-hidden="true" />
						{copied ? m.set_link_copied() : m.set_copy_link()}
					</button>
				</span>
				<span class="hint">{m.set_device_link_hint()}</span>
			</div>
		{/if}
	</div>
{/if}

{#if access}
	{@const rows = folderRows(access.folders)}
	<div class="block">
		<span class="names">
			<span>{m.set_folders()}</span>
			<span class="hint">{m.set_folders_hint()}</span>
		</span>
		{#if rows.length > 0}
			<ul class="rows">
				{#each rows as row (row.folder)}
					<li>
						<span class="folder" dir="auto">{row.folder}</span>
						{#if row.fromConfig}
							<span class="tag">{m.set_folders_config()}</span>
						{:else if session.owner}
							<button
								class="round"
								onclick={() => void removeFolder(row.folder)}
								aria-label={m.set_folder_remove_label({ folder: row.folder })}
								title={m.set_folder_remove_label({ folder: row.folder })}
							>
								<X size={15} aria-hidden="true" />
							</button>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
		{#if session.owner}
			<form class="add" onsubmit={addFolder}>
				<input
					bind:value={folder}
					dir="auto"
					placeholder={m.set_folders_label()}
					aria-label={m.set_folders_label()}
				/>
				{#if canPickFolder()}
					<button
						type="button"
						class="round"
						onclick={() => void pick()}
						aria-label={m.set_folders_pick()}
						title={m.set_folders_pick()}
					>
						<FolderOpen size={15} aria-hidden="true" />
					</button>
				{/if}
				<button type="submit" disabled={folder.trim() === ''}>{m.set_folders_add()}</button>
			</form>
		{/if}
	</div>
{/if}

{#if problem}
	<p class="problem" role="alert">{problem}</p>
{/if}

<style>
	.setting,
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

	.none {
		margin: 0;
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

	.rows .names,
	.folder {
		flex: 1;
	}

	.folder {
		overflow: hidden;
		font-family: var(--mono);
		font-size: 0.85rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tag {
		flex: none;
		padding: 0.1rem 0.6rem;
		border-radius: var(--radius-pill);
		background: var(--raised);
		color: var(--muted);
		font-size: 0.8rem;
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

	.add {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
	}

	.add input {
		flex: 1 1 14rem;
		min-width: 0;
	}

	.switch {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.link {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.field {
		display: flex;
		gap: 0.5rem;
	}

	.field input {
		flex: 1;
		min-width: 0;
		font-family: var(--mono);
		font-size: 0.8rem;
	}

	.copy {
		display: flex;
		flex: none;
		align-items: center;
		gap: 0.45rem;
		font-weight: 500;
	}

	.problem {
		margin: 0;
		padding: 0 1.25rem 0.8rem;
		color: var(--danger);
	}
</style>
