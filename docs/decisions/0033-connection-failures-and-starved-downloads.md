# 0033: One failing connection, and downloads the speed limit starves

Accepted 2026-10-14. Changes what `docs/decisions/0009` implied about errors over several connections, and adds to `0010`.

**Context:** Mathieu tested with a total limit of 256 KiB/s, an Urgent and a High download, and network and VPN changes. Downloads from an HTTP/2 server (seedr.cc) kept going back to Retrying. Three things added up:
1. Over several connections, **any one connection's error stopped every connection** of the download, which then went to Retrying, restarted them all, and could fail the same way again. Each time bytes arrived the retry count was reset, so it never ended.
2. Under a total limit, an Urgent download is served first (`0010`). The others then get nothing: their connections **kept their requests open without reading**. Servers drop such a request after a while (nginx: 60 s), which is one connection's error, so point 1 followed.
3. A download waiting for bandwidth looked stuck: "active", but at 0 B/s.

**Decision** (Mathieu's choices):
- **A connection's failure stays local.** When a connection fails with an error that may go away (`Error::is_retryable`) while others still work, the download carries on without it. Its range waits for the next free connection, and the run allows one connection fewer from then on, since the server may be limiting connections. When the last working connection fails, or on an error that won't go away (disk, a 404), the download stops as before and is retried by the usual rules (`0013`).
- **A starved connection lets go.** When the speed limit holds a connection back longer than `HttpConfig::starved_after` (20 s), it ends its request; what it already owes the limit stays owed. A connection without an answer yet waits for bandwidth **before** sending its request. So a server never waits on a request we don't read, and the download picks up again by itself when bandwidth comes back.
- **The download says it waits.** Each tick, the limiter notes a running download that owes budget and got none. After a second it's reported, and the manager sends `waitingForBandwidth: true` in a `changed` event (not saved; false again when bandwidth comes, or when the download stops). The list shows "Waiting for bandwidth" in place of the speed. The detail panel says the speed limit goes to other downloads, urgent ones first. A limit that is lifted clears what a download owed, so it goes on at once.

**Not covered:** a download over one connection (no range support, or under 2 MiB) still keeps its request while starved. Letting go would mean starting over without ranges. Small files finish fast, and the retry rules still apply.

**Found along the way** (separate fixes, same day): a retry or a new download could reuse a connection that had died silently with a network change; now each run of a download gets a client of its own, and HTTP/2 connections are pinged after 10 s of silence. Errors now carry their causes ("error decoding response body: …"), which reqwest's own message hides.

**Alternatives:**
- **Retry only the failed connection, after a wait:** keeps the connection count, but a server that limits connections would refuse it again and again.
- **Fewer connections under a tight limit** (one per download): avoids starving several streams, but a download getting its full share would lose the speed-up of several connections.
- **Show "waiting" from the UI alone** (speed 0 while a limit is set): every client would have to guess the same way, and a slow server looks the same.
