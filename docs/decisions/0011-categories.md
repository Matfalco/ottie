# 0011: Categories, with a built-in General category

Accepted 2026-10-12.

**What a category is:** a name (unique, ignoring case), an optional folder, a list of file extensions, a speed limit and a cap on running jobs (`max_concurrent`, 0 = none of its own). Categories live in the `categories` table, their extensions in `category_extensions`.

**Every job has a category.**
- A built-in **General** category (ID 1) always exists and can't be removed. Jobs that match no rule go there, and its folder is the default download folder. It starts without a folder; the daemon will set one in step 3.
- Removing another category moves its jobs to General first. Their files stay where they are.

**How a job gets its category**
- The caller can name one when adding a job, or change it later (`set_category`). That's the user's choice (`category_chosen`), and it's never overridden.
- Otherwise the extension of the file name in the URL is matched against every category's extensions (`.../Movie.MKV` → `mkv`), falling back to General. Each extension belongs to one category at most (a `UNIQUE` constraint), so a file never matches two.
- **In 2f** (`Content-Disposition` names), a job whose category was matched automatically is matched again at the first response, if nothing has been written yet. That fixes links like `download.php?id=3`.

**Folders:** a job's folder is copied when it's added: the folder the caller gave, else its category's folder, else the add fails (`NoFolder`). Changing a category's folder later doesn't move existing downloads. `set_category` also moves the job to the new category's folder, but only while nothing has been downloaded yet and no task is running. The download creates the folder if it doesn't exist.

**Limits** (completes docs/decisions/0010)
- Speed: global → category → job. A category's limit caps the total speed of its running jobs. Priorities still decide the order across categories. Each tick, the budget is shared by priority. A category that would get more than its cap gets exactly its cap, shared among its own jobs by priority, and what's left is shared again among the others (`allocate` in `throttle.rs`).
- Running jobs: the queue starts the highest-priority, oldest queued job whose category is below its own cap, as long as fewer than the global `max_concurrent` are running.

**Schema notes:** migration 4 adds a `REFERENCES` column with a non-NULL default to `jobs`. SQLite only allows that while foreign keys are off, so migrations now run with foreign keys off. Each migration checks `PRAGMA foreign_key_check` before committing (`MigrationBroken` otherwise), and foreign keys are turned on afterwards.

**Alternatives considered:** optional categories (a NULL category everywhere, plus a separate default-folder setting), matching only once at add time (wrong for links without a useful extension), and choosing the folder only at the first response, in this slice (changes the `Backend` trait before `Content-Disposition` support exists). Ordered rules allowing overlapping extensions were left out for now: one owner per extension is simpler to explain and to check.
