// Keyboard shortcuts (docs/decisions/0046): which command a key press means,
// and whether the focused control needs that key for itself (then it's
// left alone: never while typing, never a button's Space or Enter, never
// the arrows of a menu or a radio group).

import { holdsSeveral } from './batch';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** A key press, as `KeyboardEvent` has it. */
export interface Press {
	/** What the key types on this layout (`?`, `f`, `ArrowUp`). */
	key: string;
	/** Where the key sits (`Digit1`), the same on every layout. */
	code: string;
	ctrl: boolean;
	meta: boolean;
	alt: boolean;
	shift: boolean;
	/** AltGr, which some layouts need for `?` or `/` (Windows also reports
	 * it as Ctrl+Alt). */
	altGraph: boolean;
}

/** The focused element, as far as keys go. */
export interface Focus {
	/** Lower case: `input`, `button`, `body`… */
	tag: string;
	/** An input's type. */
	type: string;
	role: string | null;
	/** Text can be typed in it (`contenteditable`). */
	editable: boolean;
	/** Inside a menu, list box, radio group, slider or tab list: they use
	 * the arrows. */
	inWidget: boolean;
}

export type Step = 'up' | 'down' | 'first' | 'last';

/** What a shortcut does. */
export type Command =
	// Anywhere in the main window.
	| { kind: 'tab'; index: number }
	| { kind: 'focusAdd' }
	| { kind: 'help' }
	// In Downloads.
	| { kind: 'step'; to: Step; extend: boolean }
	| { kind: 'selectAll' }
	| { kind: 'clear' }
	| { kind: 'toggle' }
	| { kind: 'open' }
	| { kind: 'remove' }
	| { kind: 'queue'; to: 'up' | 'down' | 'top' | 'bottom' }
	| { kind: 'filters' };

/** How many tabs the digits reach. */
export const TABS = 5;

/** Input types that aren't typed in. */
const NOT_TYPED = ['checkbox', 'radio', 'button', 'submit', 'reset', 'range', 'color', 'file'];

/** Roles that answer Space and Enter like a button. */
const PRESSED = [
	'button',
	'link',
	'checkbox',
	'radio',
	'switch',
	'tab',
	'option',
	'menuitem',
	'menuitemcheckbox',
	'menuitemradio'
];

/** Whether the focused element takes typed text. */
export function isTyping(focus: Focus): boolean {
	return (
		focus.editable ||
		focus.tag === 'textarea' ||
		focus.tag === 'select' ||
		(focus.tag === 'input' && !NOT_TYPED.includes(focus.type)) ||
		focus.role === 'textbox' ||
		focus.role === 'combobox'
	);
}

/** Whether the focused element needs `press` for itself. */
export function focusUses(focus: Focus, press: Press): boolean {
	if (isTyping(focus)) {
		return true;
	}
	const pressed =
		['button', 'a', 'summary'].includes(focus.tag) ||
		(focus.role !== null && PRESSED.includes(focus.role));
	switch (press.key) {
		case ' ':
			// A checkbox or radio button ticks itself.
			return pressed || focus.tag === 'input';
		case 'Enter':
			return pressed;
		case 'ArrowUp':
		case 'ArrowDown':
		case 'ArrowLeft':
		case 'ArrowRight':
		case 'Home':
		case 'End':
			return focus.inWidget || (focus.tag === 'input' && ['radio', 'range'].includes(focus.type));
		default:
			return false;
	}
}

/** A shortcut of the main window (any tab), or null. */
export function anywhere(press: Press): Command | null {
	const command = plain(press) ? character(press.key) : null;
	if (command) {
		return command;
	}
	// The key's place, so AZERTY's 1 works without Shift.
	const digit = /^(?:Digit|Numpad)([1-9])$/.exec(press.code);
	if (digit && !press.ctrl && !press.meta && !press.alt) {
		const index = Number(digit[1]) - 1;
		return index < TABS ? { kind: 'tab', index } : null;
	}
	return null;
}

function character(key: string): Command | null {
	switch (key) {
		case '?':
			return { kind: 'help' };
		case '/':
			return { kind: 'focusAdd' };
		default:
			return null;
	}
}

/** A shortcut of the Downloads page, or null. */
export function inDownloads(press: Press): Command | null {
	const { key, shift } = press;
	if (press.ctrl || press.meta) {
		return !press.alt && !shift && key.toLowerCase() === 'a' ? { kind: 'selectAll' } : null;
	}
	if (press.alt) {
		if (key === 'ArrowUp' || key === 'ArrowDown') {
			const up = key === 'ArrowUp';
			return { kind: 'queue', to: shift ? (up ? 'top' : 'bottom') : up ? 'up' : 'down' };
		}
		return null;
	}
	switch (key) {
		case 'ArrowUp':
			return { kind: 'step', to: 'up', extend: shift };
		case 'ArrowDown':
			return { kind: 'step', to: 'down', extend: shift };
		case 'Home':
			return { kind: 'step', to: 'first', extend: shift };
		case 'End':
			return { kind: 'step', to: 'last', extend: shift };
		case 'Escape':
			return { kind: 'clear' };
		case 'Delete':
			return { kind: 'remove' };
		case ' ':
			return shift ? null : { kind: 'toggle' };
		case 'Enter':
			return shift ? null : { kind: 'open' };
		case 'f':
		case 'F':
			return { kind: 'filters' };
		default:
			return null;
	}
}

/** No Ctrl, Cmd or Alt, unless it's AltGr typing a character. */
function plain(press: Press): boolean {
	return !press.meta && (press.altGraph || (!press.ctrl && !press.alt));
}

/** What pasted text holds: the link for the link field, several (or a
 * sequence) for the list, or none (then the paste is left alone). */
export type Pasted = { kind: 'one'; link: string } | { kind: 'several' } | { kind: 'none' };

export function pasted(text: string): Pasted {
	if (holdsSeveral(text)) {
		return { kind: 'several' };
	}
	const link = text.split(/\s+/).find((word) => word.includes('://'));
	return link ? { kind: 'one', link } : { kind: 'none' };
}

/** Writes Ctrl as Cmd on Apple's systems. */
export function isApple(platform: string): boolean {
	return /Mac|iPhone|iPad/.test(platform);
}

/** A row of the list of shortcuts: its keys, and what they do. */
export interface Entry {
	/** Held with each of `keys` (`ctrl`, `shift`…). */
	with: string[];
	/** Each does it (↑ / ↓), or with `range`, the two ends of a range (1 – 5). */
	keys: string[];
	range?: boolean;
	label: () => string;
}

export interface EntryGroup {
	title: () => string;
	entries: Entry[];
}

const UP = '↑';
const DOWN = '↓';

/** The list of shortcuts (the "?" dialog, Settings). `canOpen`: Enter opens
 * a finished file (the desktop app). */
export function shortcutList(canOpen: boolean): EntryGroup[] {
	const open: Entry[] = canOpen ? [{ with: [], keys: ['enter'], label: m.shortcut_open }] : [];
	return [
		{
			title: m.shortcuts_anywhere,
			entries: [
				{ with: [], keys: ['1', String(TABS)], range: true, label: m.shortcut_tabs },
				{ with: ['ctrl'], keys: ['V'], label: m.shortcut_paste },
				{ with: [], keys: ['/'], label: m.shortcut_add },
				{ with: [], keys: ['?'], label: m.shortcut_help }
			]
		},
		{
			title: m.nav_downloads,
			entries: [
				{ with: [], keys: [UP, DOWN], label: m.shortcut_step },
				{ with: ['shift'], keys: [UP, DOWN], label: m.shortcut_extend },
				{ with: [], keys: ['home', 'end'], label: m.shortcut_ends },
				{ with: ['ctrl'], keys: ['A'], label: m.select_all },
				{ with: [], keys: ['esc'], label: m.shortcut_none },
				{ with: [], keys: ['space'], label: m.shortcut_toggle },
				...open,
				{ with: [], keys: ['delete'], label: m.remove },
				{ with: ['alt'], keys: [UP, DOWN], label: m.shortcut_queue },
				{ with: ['alt', 'shift'], keys: [UP, DOWN], label: m.shortcut_queue_ends },
				{ with: [], keys: ['F'], label: m.filters_sorting }
			]
		}
	];
}

/** Keys with a name, written in the page's language (or Apple's symbols). */
const NAMED: Record<string, (apple: boolean, locale: Locale) => string> = {
	ctrl: (apple, locale) => (apple ? '⌘' : m.key_ctrl({}, { locale })),
	alt: (apple, locale) => (apple ? '⌥' : m.key_alt({}, { locale })),
	shift: (apple, locale) => (apple ? '⇧' : m.key_shift({}, { locale })),
	space: (_, locale) => m.key_space({}, { locale }),
	enter: (_, locale) => m.key_enter({}, { locale }),
	delete: (_, locale) => m.key_delete({}, { locale }),
	esc: (_, locale) => m.key_esc({}, { locale }),
	home: (_, locale) => m.key_home({}, { locale }),
	end: (_, locale) => m.key_end({}, { locale })
};

/** What a key of the list says on its cap. */
export function keyName(key: string, apple: boolean, locale: Locale = getLocale()): string {
	return NAMED[key]?.(apple, locale) ?? key;
}

/** Containers whose items move with the arrows. */
const WIDGETS = ['menu', 'menubar', 'listbox', 'radiogroup', 'slider', 'tablist', 'grid', 'tree']
	.map((role) => `[role=${role}]`)
	.join(',');

function focusOf(target: EventTarget | null): Focus {
	if (!(target instanceof HTMLElement)) {
		return { tag: 'body', type: '', role: null, editable: false, inWidget: false };
	}
	return {
		tag: target.tagName.toLowerCase(),
		type: target instanceof HTMLInputElement ? target.type : '',
		role: target.getAttribute('role'),
		editable: target.isContentEditable,
		inWidget: target.closest(WIDGETS) !== null
	};
}

/** The key press, if it's the page's to act on: not taken by the focused
 * control or a handler already, and no dialog open (its keys are its own). */
export function pageKey(event: KeyboardEvent): Press | null {
	if (event.defaultPrevented || event.isComposing || document.querySelector('dialog[open]')) {
		return null;
	}
	const press: Press = {
		key: event.key,
		code: event.code,
		ctrl: event.ctrlKey,
		meta: event.metaKey,
		alt: event.altKey,
		shift: event.shiftKey,
		altGraph: event.getModifierState('AltGraph')
	};
	return focusUses(focusOf(event.target), press) ? null : press;
}

/** The pasted text, if the paste is the page's: not into a field, and no
 * dialog open. */
export function pageText(event: ClipboardEvent): string | null {
	if (
		event.defaultPrevented ||
		document.querySelector('dialog[open]') ||
		isTyping(focusOf(event.target))
	) {
		return null;
	}
	return event.clipboardData?.getData('text') ?? null;
}
