// Numbers and names as people read them, in the page's language: the
// browser's `Intl` knows each language's units and separators ("1.5 MB",
// "1,5 Mo").

import type { Job } from '@ottie/api-types/Job';
import { getLocale, type Locale } from './paraglide/runtime.js';

const UNITS = ['byte', 'kilobyte', 'megabyte', 'gigabyte', 'terabyte'] as const;

/** 1 KB = 1024 bytes, as most download managers and Windows count. */
function scale(bytes: number): { value: number; unit: number } {
	let value = bytes;
	let unit = 0;
	while (value >= 1024 && unit < UNITS.length - 1) {
		value /= 1024;
		unit += 1;
	}
	return { value, unit };
}

function number(value: number, unit: number, locale: Locale, intlUnit: string, long: boolean) {
	// One decimal for small numbers ("1.5 MB"), none for big ones ("234 MB").
	const digits = unit > 0 && value < 10 ? 1 : 0;
	return new Intl.NumberFormat(locale, {
		style: 'unit',
		unit: intlUnit,
		unitDisplay: long ? 'long' : 'short',
		minimumFractionDigits: digits,
		maximumFractionDigits: digits
	}).format(value);
}

/** "1,023 bytes", "1.5 kB", "234 MB" (French: "1 023 octets", "1,5 ko", "234 Mo"). */
export function formatBytes(bytes: number, locale: Locale = getLocale()): string {
	const { value, unit } = scale(bytes);
	// "1 023 byte" reads oddly; whole bytes are written out.
	return number(value, unit, locale, UNITS[unit], unit === 0);
}

/** "2.5 MB/s" (French: "2,5 Mo/s"). */
export function formatSpeed(bytesPerSec: number, locale: Locale = getLocale()): string {
	const { value, unit } = scale(bytesPerSec);
	return number(value, unit, locale, `${UNITS[unit]}-per-second`, false);
}

/** The two largest units: "45 sec", "3 min 20 sec", "2 hr 5 min" (French: "2 h 5 min"). */
export function formatDuration(seconds: number, locale: Locale = getLocale()): string {
	const part = (value: number, unit: string) =>
		new Intl.NumberFormat(locale, { style: 'unit', unit, unitDisplay: 'short' }).format(value);
	const s = Math.max(0, Math.round(seconds));
	if (s < 60) {
		return part(s, 'second');
	}
	const minutes = Math.floor(s / 60);
	if (minutes < 60) {
		const rest = s % 60;
		return rest ? `${part(minutes, 'minute')} ${part(rest, 'second')}` : part(minutes, 'minute');
	}
	const hours = Math.floor(minutes / 60);
	const rest = minutes % 60;
	return rest ? `${part(hours, 'hour')} ${part(rest, 'minute')}` : part(hours, 'hour');
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
