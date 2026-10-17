// The smallest size taken over, typed in megabytes (1024², as Ottie counts).

import { intlLocale } from '#ui/languages.js';
import type { Locale } from '#ui/paraglide/runtime.js';

const MB = 1024 ** 2;

/** "MB" in the page's language ("Mo" in French). */
export function megabytes(locale: Locale): string {
	const parts = new Intl.NumberFormat(intlLocale(locale), {
		style: 'unit',
		unit: 'megabyte',
		unitDisplay: 'short'
	}).formatToParts(1);
	return parts.find((part) => part.type === 'unit')?.value ?? 'MB';
}

/** Megabytes typed, in bytes; null unless a number from 0. */
export function parseMegabytes(text: string): number | null {
	const trimmed = text.trim().replace(',', '.');
	const number = trimmed === '' ? 0 : Number(trimmed);
	return Number.isFinite(number) && number >= 0 ? Math.round(number * MB) : null;
}

/** Bytes as megabytes in a field. */
export function toMegabytes(bytes: number): number {
	return Math.round((bytes / MB) * 100) / 100;
}
