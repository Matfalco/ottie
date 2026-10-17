// Devices with tokens of their own and the folders they may use
// (docs/decisions/0052): what Settings shows of them.

import type { AllowedFolders } from '@ottie/api-types/AllowedFolders';
import type { Device } from '@ottie/api-types/Device';
import { formatMoment } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** One folder devices may use, and whether only config.toml can change it. */
export interface FolderRow {
	folder: string;
	fromConfig: boolean;
}

/** config.toml's folders first, then those chosen in Settings, each once. */
export function folderRows(folders: AllowedFolders): FolderRow[] {
	const rows: FolderRow[] = folders.config.map((folder) => ({ folder, fromConfig: true }));
	for (const folder of folders.chosen) {
		if (!rows.some((row) => row.folder === folder)) {
			rows.push({ folder, fromConfig: false });
		}
	}
	return rows;
}

/** The folders chosen in Settings with `folder` added (trimmed, once);
 * null if there's nothing to add. */
export function withFolder(folders: AllowedFolders, folder: string): string[] | null {
	const added = folder.trim();
	if (added === '' || folders.chosen.includes(added) || folders.config.includes(added)) {
		return null;
	}
	return [...folders.chosen, added];
}

/** Full access or read-only, and when it was last used. */
export function deviceLine(device: Device, locale: Locale = getLocale()): string {
	const options = { locale };
	const access = device.readOnly
		? m.set_device_read_only({}, options)
		: m.set_device_full({}, options);
	const used =
		device.lastUsed === null
			? m.set_device_never_used({}, options)
			: m.set_device_last_used({ when: formatMoment(new Date(device.lastUsed), locale) }, options);
	return `${access} · ${used}`;
}
