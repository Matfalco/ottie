// What every page shares, set up once by the app's shell: the live session
// (one events WebSocket for all pages) and the way to act on downloads, which
// asks first when data would be lost.

import { createContext } from 'svelte';
import type { AddJob } from '@ottie/api-types/AddJob';
import type { Job } from '@ottie/api-types/Job';
import type { Action } from './bulk';
import type { Session } from './session.svelte';

export interface App {
	session: Session;
	/** The API token this page signed in with. */
	token: string;
	/** Does `action` to the jobs it applies to; asks first if that would
	 * delete downloaded data. */
	run: (action: Action, jobs: Job[]) => Promise<void>;
	/** Shows `message` in the confirmation dialog; true if the user goes ahead. */
	ask: (message: string, confirmLabel: string) => Promise<boolean>;
	/** Adds a download; resolves to true once the daemon has accepted it. */
	add: (job: AddJob) => Promise<boolean>;
}

/** `useApp()` in any page or component inside the shell. */
export const [useApp, provideApp] = createContext<App>();
