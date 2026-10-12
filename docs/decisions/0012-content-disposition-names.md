# 0012: File names from Content-Disposition, and re-matching the category

Accepted 2026-10-12.

**Where a file's name comes from**, in order:
1. `Content-Disposition`: `filename*` (RFC 8187, e.g. `UTF-8''caf%C3%A9.pdf`; UTF-8 and ISO-8859-1), else `filename`. A plain `filename` is read as UTF-8 when it's valid UTF-8, because many servers send raw UTF-8 there, and as Latin-1 otherwise. An unknown charset or broken encoding in `filename*` falls back to `filename`.
2. The last path segment of the URL the server answered from, after redirects.

Either way the name is sanitized like before, so it can't leave the target folder. It's chosen once, from the first response, and kept on resume.

The parser is about 60 lines in `filename.rs`, rather than a crate: the available crates are either unmaintained or full mail-parsing libraries, and `percent-encoding` was already a dependency.

**Re-matching the category** (completes docs/decisions/0011). A link like `download.php?id=3` is first matched as `.php`. Once the real name is known, a job whose category was matched automatically (`category_chosen` false) is matched again by the real name's extension: the category that lists it, else General. A name without an extension keeps the current category. A category the user chose is never changed.

**The folder follows only if the job follows its category.** The folder has to be decided before the `.part.ottie` file is created; moving it afterwards could cross drives. So for a job's first run, if it's saved in its category's folder, the manager hands the download a `Placement`: plain data (extension → folder, plus General's folder for everything else). The HTTP code applies it to the real name before reserving the file, and reports the folder in `Event::Started`. A job saved in a folder of the caller's choosing keeps that folder; only its category (and so its speed limit and job cap) changes.

`Placement` is used once and never saved: if Ottie restarts before the first response, it's worked out again.

**Not done:** guessing a missing extension from `Content-Type` (e.g. `download` + `application/pdf`). It can be added later if it turns out to matter.
