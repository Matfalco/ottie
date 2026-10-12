// Builds icons/icon.ico (Windows) from the drawn PNGs in res/icons, so the
// small sizes keep their simplified design instead of a scaled-down copy.
// Run it again when the icons change: pnpm --filter app icon
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const app = dirname(dirname(fileURLToPath(import.meta.url)));
const pngs = join(dirname(app), 'res', 'icons', 'png', 'app');
const sizes = [16, 32, 48, 64, 128, 256];
const images = sizes.map((size) => readFileSync(join(pngs, `ottie-icon-${size}.png`)));

// ICO: a 6-byte header, a 16-byte entry per image, then the PNG files as is.
const header = Buffer.alloc(6 + 16 * images.length);
header.writeUInt16LE(1, 2); // type: icon
header.writeUInt16LE(images.length, 4);
let offset = header.length;
images.forEach((png, i) => {
	const entry = 6 + 16 * i;
	const size = png.readUInt32BE(16); // the PNG's width, from its IHDR chunk
	if (size !== sizes[i] || png.readUInt32BE(20) !== size) {
		throw new Error(`ottie-icon-${sizes[i]}.png is ${size} pixels wide, or not square`);
	}
	header.writeUInt8(size % 256, entry); // 0 means 256
	header.writeUInt8(size % 256, entry + 1);
	header.writeUInt16LE(1, entry + 4); // colour planes
	header.writeUInt16LE(32, entry + 6); // bits per pixel
	header.writeUInt32LE(png.length, entry + 8);
	header.writeUInt32LE(offset, entry + 12);
	offset += png.length;
});
writeFileSync(join(app, 'icons', 'icon.ico'), Buffer.concat([header, ...images]));
