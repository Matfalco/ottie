// Numbers and names as people read them.

import type { Job } from '@ottie/api-types/Job';

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/** 1 KB = 1024 bytes, as most download managers and Windows count. */
export function formatBytes(bytes: number): string {
	let value = bytes;
	let unit = 0;
	while (value >= 1024 && unit < UNITS.length - 1) {
		value /= 1024;
		unit += 1;
	}
	// One decimal for small numbers ("1.5 MB"), none for big ones ("234 MB").
	const digits = unit > 0 && value < 10 ? 1 : 0;
	return `${value.toFixed(digits)} ${UNITS[unit]}`;
}

export function formatSpeed(bytesPerSec: number): string {
	return `${formatBytes(bytesPerSec)}/s`;
}

/** "45 s", "3 min 20 s", "2 h 5 min". */
export function formatDuration(seconds: number): string {
	const s = Math.max(0, Math.round(seconds));
	if (s < 60) {
		return `${s} s`;
	}
	const minutes = Math.floor(s / 60);
	if (minutes < 60) {
		const rest = s % 60;
		return rest ? `${minutes} min ${rest} s` : `${minutes} min`;
	}
	const hours = Math.floor(minutes / 60);
	const rest = minutes % 60;
	return rest ? `${hours} h ${rest} min` : `${hours} h`;
}

/** The file name once known, else the last part of the URL, else its host. */
export function displayName(job: Job): string {
	if (job.fileName) {
		return job.fileName;
	}
	try {
		const url = new URL(job.url);
		const last = url.pathname.split('/').filter(Boolean).pop();
		return last ? decodeURIComponent(last) : url.host;
	} catch {
		return job.url;
	}
}
