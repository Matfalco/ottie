# 0027: The detail panel's bar maps the file

Accepted 2026-10-13. Replaces the 48-block file map of `docs/decisions/0025`.

**Decision** (Mathieu's choice, from a mockup of three options): the detail panel has no separate file map any more. Its progress bar is the map.
- **Each range sits at its exact place** in the file, filled as far as it has arrived. A hairline in the panel's colour separates one range from the next.
- **While downloading, a thin mark** in the "arriving" colour shows where each unfinished range is being written.
- **One connection** is one range from the start, so the bar looks like a plain progress bar.
- **Under the bar:** "4 ranges" or "One connection". With several ranges, a legend follows (Received, Arriving while downloading, Left).
- **Until the size is known**, the bar has no ranges to place and shows plain progress.
- The bar takes the state's colour as before (paused, failed…), and is a little taller (0.85 rem) so short ranges stay visible.
- The rows' bars in the list stay as they were: one piece per range, sized by its share.

**Why:** with 48 blocks, a block lit only once all of it had arrived, about 2% of the file each. With several connections the map then moved in jumps, and showed far less than the ranges actually say.

**Alternatives** (both were in the mockup):
- More, smaller squares, with the columns following the panel's width: finer, but still steps, and a second bar next to the progress bar.
- A strip next to the progress bar: the same drawing, shown twice.
