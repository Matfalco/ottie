# 0040: Scheduler

Accepted 2026-10-14.

**Why:** people download at night (off-peak hours, a data cap), slow downloads down while they work or play, start one big file later, and leave the computer to turn itself off once everything is done. uGet has a weekly grid and IDM queues that start and stop at set times; Ottie replaces both.

**Decision** (Mathieu's choices):
- **Time slots**, in their own **Schedule** tab, in a list:
  - A slot applies to the **whole queue or one category**.
  - It **repeats** on chosen weekdays, or happens **once** on a date (and removes itself after it ends). It runs from one time to another; an end before the start means the next day.
  - It does any of: **allow or stop** downloads, a **speed limit** (the total one for the queue, the category's for a category), a **max concurrent**.
  - When a slot stops downloads, the ones running **pause and carry on when it ends**; a slot can instead **let them finish** (only new ones don't start).
  - **The slot higher in the list wins** where two apply to the same thing at the same time, for each of the three effects; the tab warns about such conflicts. A queue slot and a category slot both apply (a stopped queue stops every category).
  - **Outside every slot, the usual settings apply.** A slot never changes them; it only overrides them while it lasts. As everywhere, a lower max concurrent never stops running downloads.
- **A download's own start and stop time:** it waits until its start time, then joins the queue at its place; at its stop time it pauses (if it hasn't started by then, too). Each time is used once, then cleared.
- **A new state, `Scheduled`:** a download waiting for its start time, or held by a slot.
  - Queued ↔ Scheduled; Connecting | Downloading | Retrying → Scheduled (a slot stops it); Scheduled → Paused | Cancelled; Paused | Failed → Scheduled (resumed while held).
  - It's saved like the other states; at start, the daemon checks again whether each one is still held.
  - In the Downloads page it's one of the Waiting group's statuses, shown with its start time or the slot holding it.
  - Starting a scheduled download by hand clears its start time; if a slot holds it, it runs anyway, and slots leave it alone until it pauses, finishes or fails.
  - Pausing it clears its start time too, so the tray's "Pause all" leaves scheduled downloads alone ("Resume all" couldn't give their times back).
- **When everything has finished** (desktop app): quit Ottie, sleep, hibernate or shut down.
  - Chosen **once** (from the tray menu and the Downloads page; cleared after it runs) or **permanently** in Settings. Both are this computer's settings, like notifications (`docs/decisions/0035`).
  - Before sleep, hibernate or shut down, a **countdown** with Cancel; its length is a setting (60 s at first).
  - **"Finished"** means nothing is queued, connecting, downloading, retrying or scheduled. A setting leaves scheduled downloads out ("nothing left to run"). Paused and failed downloads never count.
- **Times are those of the computer running the daemon** (the tab says which when the daemon is remote), and follow its wall clock across daylight-saving changes: a slot starting in an hour that's skipped starts at the jump; an hour that repeats counts once. A download's own times are exact instants (saved in UTC).

**Alternatives:**
- uGet's weekly grid of hours: simple, but too rigid (whole hours, one scope, one effect per hour).
- Lower in the list wins, the most restrictive wins, or overlaps refused: Mathieu preferred list order with a warning.
- A held download as Queued with a reason (only the times saved, the hold worked out from the clock), or as Paused with who paused it: fewer changes, but a state of its own is explicit and checked by the state machine (`JobState::can_become`).
- A start time only per download: Mathieu also wanted a stop time.
- "Finished" ignoring scheduled downloads by default: chosen as the setting, not the default.
