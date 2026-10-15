// The desktop app's "when everything has finished" (docs/decisions/0040),
// live in a page: asked once, then kept up to date by the app, which tells
// every window when it changes. The clock ticks while a countdown runs.

import { secondsLeft } from './finish.js';
import { finishing, followFinishing, type Finishing } from './window.js';

/** How often the countdown's seconds are worked out again. */
const TICK = 250;

export class LiveFinishing {
	/** Null until the app has answered. */
	current: Finishing | null = $state(null);
	/** The app didn't answer. */
	failed = $state(false);
	#now = $state(Date.now());
	#timer: ReturnType<typeof setInterval> | null = null;
	#unfollow: (() => void) | null = null;

	/** Seconds left in the countdown, while one runs. */
	get left(): number | null {
		const running = this.current?.running;
		return running ? secondsLeft(running.endsAt, this.#now) : null;
	}

	start() {
		this.#unfollow = followFinishing((next) => this.#show(next));
		finishing().then(
			(next) => this.#show(next),
			() => (this.failed = true)
		);
	}

	stop() {
		this.#unfollow?.();
		this.#unfollow = null;
		this.#clock(false);
	}

	/** Makes a change through the app; false if it refused. */
	async change(change: () => Promise<Finishing>): Promise<boolean> {
		try {
			this.#show(await change());
			return true;
		} catch {
			return false;
		}
	}

	#show(next: Finishing) {
		this.current = next;
		this.failed = false;
		this.#now = Date.now();
		this.#clock(next.running !== null);
	}

	#clock(on: boolean) {
		if (on && this.#timer === null) {
			this.#timer = setInterval(() => (this.#now = Date.now()), TICK);
		} else if (!on && this.#timer !== null) {
			clearInterval(this.#timer);
			this.#timer = null;
		}
	}
}
