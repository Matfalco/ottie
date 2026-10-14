# 0036: Several links at once

Accepted 2026-10-14.

**Why:** adding links one at a time is slow when there are dozens: a list from a forum post, an email, a page of images, a numbered series. uGet has batch adding (a list, a URL sequence) and Ottie replaces it.

**Decision** (Mathieu's choices):
- **Each link is its own download**, queued as if added alone (last in the queue, in the order given). Nothing remembers that they came together: selecting them in Downloads and the bulk actions already act on several.
- **Any text can be pasted**: one link per line, an email, a web page. Anything of the form `scheme://…` counts as a link, ending at a space, a quote or an angle bracket, without the punctuation of the sentence around it (a final full stop or comma, a closing bracket the link doesn't open). Each link counts once. Links that can't be added (`ftp://`, malformed) are listed too, with why.
- **The daemon reads the text**, not the page (`POST /jobs/preview`): the web UI, the desktop app, the browser extension, a later `ottie add` and Android get the same answer. For each link it gives the file name the link suggests, the category it would get, and whether a download of that same link is already on the list. It adds nothing.
- **A dialog shows the list** when several links are pasted into the link field (or from its "several links" button, or by importing a text file into it). Each link has a tick box and its own category menu. New links come ticked, except those already on the list and those that can't be added. Below: a category for all (Automatic, by each file's type, or one category) and a folder for all.
- **Those that can be added are** (`POST /jobs/batch`): one request, one database transaction, one `changed` event each. The answer gives each one's ID or why it was refused (the error `POST /jobs` would give). The dialog then keeps only the refused ones, each saying why.
- **A cap, as a setting** (`maxBatch`, 10,000 by default, 10 to 100,000): more links at once are all refused (`tooManyLinks`, with the count), by both endpoints. The two endpoints take bodies up to 64 MB (others keep axum's 2 MB).
- **Next (5k): sequences in one link**, curl-style: `[001-120]` (zero-padded as written), `[a-z]`, `{small,large}`, `[0-100:10]` (a step), several in one link giving every combination. The daemon expands them in the same preview, counting before expanding so the cap refuses a huge one at once. Only well-formed patterns count (`http://[::1]/` and `a[]=1` stay as they are); `\[` escapes. A missing number (404) is an ordinary failed download.

**Alternatives:**
- Expanding in the UI and sending a plain list: simpler at first, but every other client (extension, CLI, Android) would have to do it again.
- All or nothing: one bad link would hold back the rest. Adding the good ones and reporting the others lets the dialog show exactly what's left.
- One link per line only: stricter, but pasting from an email or a page is the common case.
- Always skipping links already on the list: a download can rightly be wanted twice (the file changed), so it's unticked, not refused.
- A fixed cap: a setting costs little, and some people do add tens of thousands of images.
- For sequences, uGet's and IDM's `*` placeholder with From/To/Digits fields: easier to find, but one variable only, a separate form, and `*` can be in URLs too.
