import { describe, expect, it } from 'vitest';
import { fileKind } from './filetypes';

describe('fileKind', () => {
	it('goes by the last extension, ignoring case', () => {
		expect(fileKind('Movie Night.MKV')).toBe('video');
		expect(fileKind('backup.tar.gz')).toBe('archive');
		expect(fileKind('ubuntu-24.04.iso')).toBe('program');
		expect(fileKind('notes.pdf')).toBe('document');
	});

	it('knows nothing about names without a known extension', () => {
		expect(fileKind('README')).toBe('other');
		expect(fileKind('.bashrc')).toBe('other');
		expect(fileKind('data.bin')).toBe('other');
	});
});
