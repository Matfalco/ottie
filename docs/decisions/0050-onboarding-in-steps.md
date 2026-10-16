# 0050: Onboarding in steps

Accepted 2026-10-16. Builds on `docs/decisions/0020` (first-start setup) and `0039` (starting at sign-in).

**Why:** the first-start setup was one long page: the downloads folder, the starter categories and, in the desktop app, sign-in and the clipboard. More choices are worth making at the start (the language before reading anything else, the look, closing the window, notifications), and one page with all of them would be a second Settings page. Each should be easy to pass by.

**Decision** (Mathieu's choices):
- **One step at a time,** in place of the app as before, under "Welcome to Ottie": a bar of numbered dots (ticked once passed, its step names for screen readers), then the step in a card ("Step 2 of 6" above its title), then Skip the setup at the start, and Back, Skip this step and Next (Start on the last) at the end.
- **The steps**, everywhere: **Language and look** (the language picker, light or dark, the colours); **Downloads** (the downloads folder, how many at once, a file already there, `0049`); **Categories** (the starter categories, as before). In the desktop app also: **Starting and closing** (what starts at sign-in, Ottie by default, and closing to the tray), **Clipboard** (watching it, off unless chosen, and what a copied link does) and **Notifications** (when, failed ones, the sounds).
- **Saved at each step:** Next saves what the step shows; Skip this step leaves things as they were. The language and look apply as they're clicked, as in Settings. Going past a step saves it as the daemon's `setupStep` (`PATCH /settings`, with that step's settings in the same change), so a setup stopped halfway (a window closed, a language chosen: the page reloads) carries on after it, in whichever front end opens next; one without that step (a browser after the desktop app's steps) opens on its last. Skip the setup keeps what was saved and ends it: `setupDone` true, `setupStep` cleared.
- **Run again** from Settings (a button beside its title): in that window only, from the first step, showing the settings as they are; nothing about it is saved but the settings it changes, and closing it is just closing. Starter categories already made aren't offered again.
- `setupStep` is one of the steps' names (`look`, `downloads`, `categories`, `startup`, `clipboard`, `notifications`); another is refused (`invalidBody`). The engine keeps it as text (migration 19), so a step added later needs no migration; one it doesn't know counts as none.

**Alternatives:**
- **Steps listed on the side,** to jump to any: easier to go back, but a second layout to keep working at 720 × 480 and on a phone.
- **A dialog over the Dashboard:** lighter, but cramped on a phone, and the category editor would be a dialog in a dialog.
- **Saving everything at the end** (as before): simpler, but a language chosen first would only show at the end, and closing halfway lost it all.
- **The step remembered by each front end** (browser storage): no API change, but the desktop app and a browser would each start over.
