// Ottie on this computer, through its native messaging host (`ottie`
// itself, docs/decisions/0055): it signs this browser in (a device token
// of its own) and starts Ottie when it isn't running. Set up by the
// desktop app, or by `ottie extension install`.

import type { Pairing } from './settings';

export const HOST = 'io.github.matfalco.ottie';

export type HostAnswer =
	| { type: 'paired'; address: string; token: string }
	| { type: 'started'; address: string }
	| { type: 'error'; message: string };

/** The host isn't set up for this browser (or Ottie isn't installed). */
export class NoHost extends Error {
	constructor(reason: string) {
		super(`Ottie's host for this browser: ${reason}`);
	}
}

/** The host answered, but couldn't do it (its English reason). */
export class HostFailed extends Error {}

export function parseAnswer(data: unknown): HostAnswer | null {
	if (typeof data !== 'object' || data === null) {
		return null;
	}
	const { type, address, token, message } = data as Record<string, unknown>;
	if (type === 'paired' && typeof address === 'string' && typeof token === 'string') {
		return { type, address, token };
	}
	if (type === 'started' && typeof address === 'string') {
		return { type, address };
	}
	if (type === 'error' && typeof message === 'string') {
		return { type, message };
	}
	return null;
}

/** What the browser is called in Ottie's list of devices. */
export function browserName(userAgent: string, brave = false): string {
	if (brave) {
		return 'Brave';
	}
	const known: [RegExp, string][] = [
		[/Firefox\//, 'Firefox'],
		[/Edg\//, 'Microsoft Edge'],
		[/OPR\//, 'Opera'],
		[/Vivaldi\//, 'Vivaldi'],
		[/Chrome\//, 'Chrome']
	];
	return known.find(([pattern]) => pattern.test(userAgent))?.[1] ?? 'Browser';
}

async function ask(message: object): Promise<HostAnswer> {
	let reply: unknown;
	try {
		reply = await chrome.runtime.sendNativeMessage(HOST, message);
	} catch (error) {
		throw new NoHost(error instanceof Error ? error.message : String(error));
	}
	const answer = parseAnswer(reply);
	if (answer === null) {
		throw new HostFailed('An answer Ottie’s host shouldn’t give.');
	}
	if (answer.type === 'error') {
		throw new HostFailed(answer.message);
	}
	return answer;
}

/** Signs this browser in to Ottie on this computer, starting it if needed. */
export async function pairHere(): Promise<Pairing> {
	const brave = 'brave' in navigator;
	const answer = await ask({ type: 'pair', browser: browserName(navigator.userAgent, brave) });
	if (answer.type !== 'paired') {
		throw new HostFailed('Ottie’s host didn’t sign this browser in.');
	}
	return { address: answer.address, token: answer.token, byHost: true };
}

/** Starts Ottie on this computer if it isn't running; whether it now is. */
export async function startHere(): Promise<boolean> {
	try {
		return (await ask({ type: 'start' })).type === 'started';
	} catch {
		return false;
	}
}

/** Whether Ottie's address is on this computer (only that one can be
 * started from here). */
export function onThisComputer(address: string): boolean {
	try {
		const host = new URL(address).hostname;
		return host === '127.0.0.1' || host === 'localhost' || host === '[::1]';
	} catch {
		return false;
	}
}
