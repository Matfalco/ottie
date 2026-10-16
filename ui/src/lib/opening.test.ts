import { describe, expect, it } from 'vitest';
import { canOpen, openProblem, toOpen } from './opening';
import { job, plain } from './testing';

const done = job({ id: 'a', state: 'completed', fileName: 'setup.exe' });
const other = job({ id: 'b', state: 'completed', fileName: 'notes.txt' });
const running = job({ id: 'c', state: 'downloading', fileName: 'film.mkv' });

describe('opening a file', () => {
	it('opens finished files only', () => {
		expect(canOpen(done)).toBe(true);
		expect(canOpen(running)).toBe(false);
		expect(canOpen(job({ id: 'd', state: 'completed', fileName: null }))).toBe(false);
	});

	it('opens with Enter the one finished download selected', () => {
		expect(toOpen([done])).toBe(done);
		expect(toOpen([done, other])).toBe(null);
		expect(toOpen([running])).toBe(null);
		expect(toOpen([])).toBe(null);
	});

	it('says why it could not, in both languages', () => {
		expect(openProblem('gone', done, false, 'en')).toBe(
			'“setup.exe” isn’t there any more: it was moved or deleted.'
		);
		expect(openProblem('gone', done, true, 'en')).toBe(
			'The folder of “setup.exe” isn’t there any more.'
		);
		expect(plain(openProblem('failed', done, false, 'fr'))).toBe(
			'Ottie n’a pas pu ouvrir « setup.exe ».'
		);
	});
});
