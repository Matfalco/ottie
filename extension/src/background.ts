// The extension's background (a service worker in Chromium-based browsers,
// a background script in Firefox): takes the browser's downloads over and
// hands them to Ottie (docs/decisions/0055).
//
// A download is never dropped: the browser's goes on until Ottie has
// accepted it, and is only then cancelled and taken off the browser's list.
// If the browser finished it first, it keeps it and Ottie's is removed.

import type { AddJob } from '@ottie/api-types/AddJob';
import { cookieHeader } from './lib/cookies';
import { addJob, removeJob, Unauthorized, Unreachable } from './lib/daemon';
import { onThisComputer, pairHere, startHere } from './lib/host';
import { RecentPosts } from './lib/posts';
import { loadSettings, saveSettings, type Pairing } from './lib/settings';
import { skipReason } from './lib/takeover';

const posts = new RecentPosts();

chrome.webRequest.onBeforeRequest.addListener(
	(details) => {
		if (details.method === 'POST') {
			posts.note(details.url, Date.now());
		}
		return undefined;
	},
	{ urls: ['<all_urls>'], types: ['main_frame', 'sub_frame'] }
);

// Installed where Ottie is: it signs in by itself (docs/decisions/0055).
// Without Ottie's host, the options page asks for a sign-in link.
chrome.runtime.onInstalled.addListener(() => {
	void signInHere();
});

async function signInHere(): Promise<void> {
	const settings = await loadSettings();
	if (settings.pairing !== null) {
		return;
	}
	try {
		await saveSettings({ pairing: await pairHere() });
	} catch {
		// No host here: a link it is, from the options page.
	}
}

chrome.downloads.onCreated.addListener((item) => {
	void takeOver(item);
});

async function takeOver(item: chrome.downloads.DownloadItem): Promise<void> {
	const settings = await loadSettings();
	const skip = skipReason(item, settings, (url) => posts.has(url, Date.now()));
	if (skip !== null || settings.pairing === null) {
		return;
	}
	let pairing = settings.pairing;
	let id: string;
	try {
		const job: AddJob = {
			url: item.url,
			cookie: (await cookiesFor(item)) ?? undefined,
			referrer: item.referrer === '' ? undefined : item.referrer,
			userAgent: navigator.userAgent
		};
		[pairing, id] = await handOver(pairing, job);
	} catch (error) {
		// Unreachable, refused or signed out: the browser carries on.
		console.warn('Ottie did not take a download:', error);
		return;
	}
	await setBrowserUi(false);
	try {
		await chrome.downloads.cancel(item.id);
		const [now] = await chrome.downloads.search({ id: item.id });
		if (now?.state === 'complete') {
			await removeJob(pairing, id).catch(() => undefined);
			return;
		}
		await chrome.downloads.erase({ id: item.id });
	} catch (error) {
		console.warn('The browser kept a download Ottie took too:', error);
	} finally {
		await setBrowserUi(true);
	}
}

/** Adds the job; where Ottie's host set this browser up, starts Ottie if
 * it doesn't answer, or signs in again if it refused the token, then tries
 * once more. The browser's download carries on meanwhile. */
async function handOver(pairing: Pairing, job: AddJob): Promise<[Pairing, string]> {
	try {
		return [pairing, await addJob(pairing, job)];
	} catch (error) {
		if (error instanceof Unreachable && onThisComputer(pairing.address) && (await startHere())) {
			return [pairing, await addJob(pairing, job)];
		}
		if (error instanceof Unauthorized && pairing.byHost === true) {
			const fresh = await pairHere();
			await saveSettings({ pairing: fresh });
			return [fresh, await addJob(fresh, job)];
		}
		throw error;
	}
}

/** The cookies the browser would send with the download's link, from the
 * store it came from (Firefox's containers have their own). */
async function cookiesFor(item: chrome.downloads.DownloadItem): Promise<string | null> {
	const storeId = (item as { cookieStoreId?: string }).cookieStoreId;
	try {
		const cookies = await chrome.cookies.getAll({ url: item.url, storeId });
		return cookieHeader(cookies);
	} catch {
		return null;
	}
}

/** Chromium-based browsers only: hides their download bubble while one
 * is taken over, so it doesn't open for a download that leaves at once. */
async function setBrowserUi(enabled: boolean): Promise<void> {
	try {
		await chrome.downloads.setUiOptions?.({ enabled });
	} catch {
		// Firefox has no such thing; nothing to do.
	}
}
