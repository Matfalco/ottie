// The fonts for Chinese, Japanese, Korean and Hindi, cut to the characters
// our translations use (docs/decisions/0041): the whole fonts would weigh
// ~18 MB. Run `pnpm --filter ui fonts` after changing one of these languages'
// messages; a test says when a character is missing. Writes src/fonts/: one
// .woff2 per language and faces.css, whose unicode-range lists what each holds.

import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import * as prettier from 'prettier';
import subsetFont from 'subset-font';

const UI = fileURLToPath(new URL('..', import.meta.url));
const OUT = `${UI}src/fonts/`;
/** The downloaded fonts, kept between runs (node_modules is git-ignored). */
const CACHE = `${UI}node_modules/.cache/ottie-fonts/`;

/** Google Fonts' copies of Noto (SIL Open Font License), at a fixed commit. */
const SOURCE =
	'https://raw.githubusercontent.com/google/fonts/2bc5d431b584d89e646214bf6cfa75494e76ceea/ofl/';

const FONTS = [
	{ locale: 'zh-Hans', family: 'Noto Sans SC', source: 'notosanssc/NotoSansSC[wght].ttf' },
	{ locale: 'zh-Hant', family: 'Noto Sans TC', source: 'notosanstc/NotoSansTC[wght].ttf' },
	{ locale: 'ja', family: 'Noto Sans JP', source: 'notosansjp/NotoSansJP[wght].ttf' },
	{ locale: 'ko', family: 'Noto Sans KR', source: 'notosanskr/NotoSansKR[wght].ttf' },
	{
		locale: 'hi',
		family: 'Noto Sans Devanagari',
		source: 'notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf',
		// Only the normal width; the weights stay variable.
		axes: { wdth: 100 }
	}
];

/** Printable ASCII: Latin file names, numbers and units in the same font. */
const ASCII = String.fromCharCode(...Array.from({ length: 95 }, (_, i) => 0x20 + i));

/** What the page may write through `Intl` in `locale` (src/lib/format.ts,
 * schedule.ts, slots.ts), sampled widely: units, numbers, dates, weekdays,
 * months, hours. */
function intlText(locale) {
	const parts = [];
	const numbers = [0, 1, 2.5, 12, 1234, 1234567.5];
	const units = ['byte', 'kilobyte', 'megabyte', 'gigabyte', 'terabyte'].flatMap((unit) => [
		unit,
		`${unit}-per-second`
	]);
	for (const unit of [...units, 'second', 'minute', 'hour', 'day']) {
		for (const unitDisplay of ['short', 'long', 'narrow']) {
			const format = new Intl.NumberFormat(locale, { style: 'unit', unit, unitDisplay });
			parts.push(...numbers.map((n) => format.format(n)));
		}
	}
	parts.push(new Intl.NumberFormat(locale, { style: 'percent' }).format(0.8));
	parts.push(...numbers.map((n) => new Intl.NumberFormat(locale).format(n)));
	const styles = [
		{ dateStyle: 'full', timeStyle: 'medium' },
		{ dateStyle: 'long', timeStyle: 'short' },
		{ dateStyle: 'medium' },
		{ dateStyle: 'short', timeStyle: 'medium' },
		{ timeStyle: 'short' },
		{ weekday: 'long' },
		{ weekday: 'short' },
		{ weekday: 'narrow' },
		{ weekday: 'short', day: 'numeric', month: 'short' },
		{ month: 'long' },
		{ hour: 'numeric' }
	].map((style) => new Intl.DateTimeFormat(locale, style));
	// Every month, weekday and hour of 2024.
	for (let month = 0; month < 12; month++) {
		for (let day = 1; day <= 7; day++) {
			const date = new Date(2024, month, day, (month * 7 + day) % 24, 30);
			parts.push(...styles.map((style) => style.format(date)));
		}
	}
	return parts.join('');
}

/** Each language's name in the picker, from src/lib/languages.ts. */
async function nameOf(locale) {
	const source = await readFile(`${UI}src/lib/languages.ts`, 'utf8');
	const found = source.match(new RegExp(`\\s'?${locale}'?: '([^']+)'`));
	if (!found) {
		throw new Error(`no name for ${locale} in languages.ts`);
	}
	return found[1];
}

/** The characters `font` must draw, in code point order. */
async function charactersOf({ locale }) {
	const messages = JSON.parse(await readFile(`${UI}messages/${locale}.json`, 'utf8'));
	const text = [...Object.values(messages), await nameOf(locale), intlText(locale), ASCII].join('');
	const points = new Set([...text].map((c) => c.codePointAt(0)).filter((p) => p >= 0x20));
	return [...points].sort((a, b) => a - b);
}

/** "U+20-7E, U+4E00, …": the code points as CSS ranges. */
function unicodeRange(points) {
	const ranges = [];
	for (const point of points) {
		const last = ranges.at(-1);
		if (last && last[1] === point - 1) {
			last[1] = point;
		} else {
			ranges.push([point, point]);
		}
	}
	const hex = (p) => p.toString(16).toUpperCase();
	return ranges
		.map(([from, to]) => (from === to ? `U+${hex(from)}` : `U+${hex(from)}-${hex(to)}`))
		.join(', ');
}

/** The whole font, downloaded once. */
async function download(source) {
	const name = source.split('/').pop();
	try {
		return await readFile(CACHE + name);
	} catch {
		console.log(`Downloading ${name}…`);
		const response = await fetch(SOURCE + source);
		if (!response.ok) {
			throw new Error(`${name}: HTTP ${response.status}`);
		}
		const font = Buffer.from(await response.arrayBuffer());
		await mkdir(CACHE, { recursive: true });
		await writeFile(CACHE + name, font);
		return font;
	}
}

const faces = [
	'/* Made by scripts/fonts.mjs (`pnpm --filter ui fonts`): don’t edit. Each font',
	'   holds only what its language’s page writes (docs/decisions/0041). */'
];
await mkdir(OUT, { recursive: true });
for (const font of FONTS) {
	const points = await charactersOf(font);
	const file = `${font.family.toLowerCase().replaceAll(' ', '-')}.woff2`;
	const cut = await subsetFont(await download(font.source), String.fromCodePoint(...points), {
		targetFormat: 'woff2',
		// Keep the copyright, licence and its address in the font itself.
		preserveNameIds: [0, 13, 14],
		variationAxes: font.axes
	});
	await writeFile(OUT + file, cut);
	console.log(`${file}: ${points.length} characters, ${Math.round(cut.length / 1024)} KiB`);
	faces.push(
		'',
		`/* ${font.locale} */`,
		'@font-face {',
		`font-family: 'Ottie ${font.family}';`,
		'font-style: normal;',
		'font-weight: 100 900;',
		'font-display: swap;',
		`src: url('./${file}') format('woff2');`,
		`unicode-range: ${unicodeRange(points)};`,
		'}'
	);
}
const css = faces.join('\n') + '\n';
const options = await prettier.resolveConfig(OUT + 'faces.css');
await writeFile(OUT + 'faces.css', await prettier.format(css, { ...options, parser: 'css' }));
