import { describe, expect, it } from 'vitest';
import {
	anywhere,
	focusUses,
	inDownloads,
	isApple,
	keyName,
	shortcutList,
	pasted,
	type Focus,
	type Press
} from './shortcuts';

/** A key with no modifiers; `code` defaults to something unrelated. */
const press = (key: string, more: Partial<Press> = {}): Press => ({
	key,
	code: 'KeyX',
	ctrl: false,
	meta: false,
	alt: false,
	shift: false,
	altGraph: false,
	...more
});

const focus = (tag: string, more: Partial<Focus> = {}): Focus => ({
	tag,
	type: '',
	role: null,
	editable: false,
	inWidget: false,
	...more
});

const BODY = focus('body');

describe('shortcuts anywhere', () => {
	it('goes to a tab with 1 to 5, by the key’s place', () => {
		expect(anywhere(press('1', { code: 'Digit1' }))).toEqual({ kind: 'tab', index: 0 });
		// AZERTY: the same key types "&" without Shift.
		expect(anywhere(press('&', { code: 'Digit1' }))).toEqual({ kind: 'tab', index: 0 });
		expect(anywhere(press('5', { code: 'Numpad5' }))).toEqual({ kind: 'tab', index: 4 });
		expect(anywhere(press('6', { code: 'Digit6' }))).toBe(null);
		// Ctrl+1 is the browser's.
		expect(anywhere(press('1', { code: 'Digit1', ctrl: true }))).toBe(null);
	});

	it('shows the list with ?, and the link field with /', () => {
		expect(anywhere(press('?', { shift: true }))).toEqual({ kind: 'help' });
		expect(anywhere(press('/'))).toEqual({ kind: 'focusAdd' });
		// Typed with AltGr (Ctrl+Alt on Windows) on some layouts.
		expect(anywhere(press('?', { ctrl: true, alt: true, altGraph: true }))).toEqual({
			kind: 'help'
		});
		expect(anywhere(press('/', { ctrl: true }))).toBe(null);
		expect(anywhere(press('f'))).toBe(null);
	});
});

describe('shortcuts in Downloads', () => {
	it('moves the selection with the arrows, Home and End, Shift extending it', () => {
		expect(inDownloads(press('ArrowDown'))).toEqual({ kind: 'step', to: 'down', extend: false });
		expect(inDownloads(press('ArrowUp', { shift: true }))).toEqual({
			kind: 'step',
			to: 'up',
			extend: true
		});
		expect(inDownloads(press('Home'))).toEqual({ kind: 'step', to: 'first', extend: false });
		expect(inDownloads(press('End', { shift: true }))).toEqual({
			kind: 'step',
			to: 'last',
			extend: true
		});
	});

	it('moves in the queue with Alt, to either end with Alt+Shift', () => {
		expect(inDownloads(press('ArrowUp', { alt: true }))).toEqual({ kind: 'queue', to: 'up' });
		expect(inDownloads(press('ArrowDown', { alt: true, shift: true }))).toEqual({
			kind: 'queue',
			to: 'bottom'
		});
		expect(inDownloads(press('f', { alt: true }))).toBe(null);
	});

	it('knows the file manager’s keys', () => {
		expect(inDownloads(press('a', { ctrl: true }))).toEqual({ kind: 'selectAll' });
		expect(inDownloads(press('A', { meta: true }))).toEqual({ kind: 'selectAll' });
		expect(inDownloads(press('Escape'))).toEqual({ kind: 'clear' });
		expect(inDownloads(press('Delete'))).toEqual({ kind: 'remove' });
		expect(inDownloads(press(' '))).toEqual({ kind: 'toggle' });
		expect(inDownloads(press('Enter'))).toEqual({ kind: 'open' });
		expect(inDownloads(press('F', { shift: true }))).toEqual({ kind: 'filters' });
		expect(inDownloads(press('f', { ctrl: true }))).toBe(null);
		expect(inDownloads(press('x'))).toBe(null);
	});
});

describe('focus', () => {
	it('leaves every key to a field being typed in', () => {
		for (const typed of [
			focus('input', { type: 'url' }),
			focus('input', { type: 'text' }),
			focus('textarea'),
			focus('select'),
			focus('div', { editable: true })
		]) {
			expect(focusUses(typed, press('1', { code: 'Digit1' }))).toBe(true);
			expect(focusUses(typed, press('ArrowDown'))).toBe(true);
			expect(focusUses(typed, press('Delete'))).toBe(true);
		}
	});

	it('leaves Space and Enter to buttons and links, Space to checkboxes', () => {
		const checkbox = focus('input', { type: 'checkbox' });
		expect(focusUses(checkbox, press(' '))).toBe(true);
		expect(focusUses(checkbox, press('Enter'))).toBe(false);
		for (const pressed of [focus('button'), focus('a'), focus('div', { role: 'switch' })]) {
			expect(focusUses(pressed, press(' '))).toBe(true);
			expect(focusUses(pressed, press('Enter'))).toBe(true);
			expect(focusUses(pressed, press('ArrowDown'))).toBe(false);
			expect(focusUses(pressed, press('?'))).toBe(false);
		}
	});

	it('leaves the arrows to menus, radio buttons and sliders', () => {
		expect(focusUses(focus('button', { inWidget: true }), press('ArrowDown'))).toBe(true);
		expect(focusUses(focus('input', { type: 'radio' }), press('ArrowUp'))).toBe(true);
		expect(focusUses(focus('input', { type: 'range' }), press('Home'))).toBe(true);
		expect(focusUses(BODY, press('ArrowDown'))).toBe(false);
		expect(focusUses(BODY, press(' '))).toBe(false);
	});
});

describe('pasting', () => {
	it('tells one link from several, and from none', () => {
		expect(pasted('  https://example.com/a.zip\n')).toEqual({
			kind: 'one',
			link: 'https://example.com/a.zip'
		});
		expect(pasted('see https://example.com/a.zip')).toEqual({
			kind: 'one',
			link: 'https://example.com/a.zip'
		});
		expect(pasted('https://example.com/a.zip https://example.com/b.zip')).toEqual({
			kind: 'several'
		});
		expect(pasted('https://example.com/[1-3].zip')).toEqual({ kind: 'several' });
		expect(pasted('just words')).toEqual({ kind: 'none' });
	});
});

describe('the list', () => {
	it('writes Cmd on Apple’s systems', () => {
		expect(isApple('MacIntel')).toBe(true);
		expect(isApple('Win32')).toBe(false);
		expect(keyName('ctrl', true, 'en')).toBe('⌘');
	});

	it('names keys in the page’s language', () => {
		expect(keyName('ctrl', false, 'en')).toBe('Ctrl');
		expect(keyName('shift', false, 'fr')).toBe('Maj');
		expect(keyName('delete', false, 'fr')).toBe('Suppr');
		expect(keyName('F', false, 'fr')).toBe('F');
	});

	it('lists Enter only where it opens files', () => {
		const keys = (canOpen: boolean) =>
			shortcutList(canOpen).flatMap((group) => group.entries.flatMap((entry) => entry.keys));
		expect(keys(true)).toContain('enter');
		expect(keys(false)).not.toContain('enter');
	});
});
