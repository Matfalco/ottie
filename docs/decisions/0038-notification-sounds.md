# 0038: Notification sounds

Accepted 2026-10-14.

**Why:** on Windows, Ottie's notifications made no sound: a toast without a sound name is made silent by `notify-rust` (under `tauri-plugin-notification`). And people want to tell a finished download from a failed one by ear, with sounds that feel like Ottie's.

**Decision** (Mathieu's choices):
- **Sets of two sounds:** Ottie has its own sound sets, each with a sound for a finished download and one for a failed download that goes with it: Marimba, Log, Driftwood, Shell, Stream, Splash, Burrow, Tide, Holt, Kelp (`app/sounds/`; how each was made, and its licence, in `app/sounds/README.md`). Mathieu chose them by ear among about 120: recordings from CC0 packs, sounds made from real instrument recordings, and sounds synthesized for Ottie.
- **Settings → Notifications → Sounds:** **System default** (the default), **Silent**, one of the sets, or **Custom**, which shows two pickers to choose the sound of each kind (System default, Silent, or any set's sound). Custom starts from what was heard until then. The system's other sounds aren't offered: the list is the same on every OS.
- **System default** asks the toast for the system's sound (on Windows the sound name `Default`; elsewhere the notification server decides). **Silent** and Ottie's sounds make the toast silent; the app plays Ottie's sound itself. Windows lets a toast from an app outside the Store play only the system's sounds, never a file.
- **One sound for notifications shown together,** a failure's first.
- **A choice plays as it's made,** and ▶ plays it again, to compare: a set plays both its sounds, half a second apart. On Windows that includes the system's: the app plays the file set for notifications in the Control Panel's Sounds (`AppEvents\…\Notification.Default\.Current`, what a toast's `Default` plays). Elsewhere the notification server chooses, so it can't.
- **As loud as the system's, a little less:** every sound is 3 dB quieter than Windows' notification sound, measured as ITU-R BS.1770 loudness over the loudest 200 ms, with peaks at most -6 dBFS.
- **Ogg Vorbis** (48 kHz; mono, or stereo where the sound has room in it): about 240 KB for the 20 files, a tenth of WAV.
- **Saved by name in `app.json`:** `sounds` (`system`, `silent`, a set's ID or `custom`), and with `custom`, `soundFinished` and `soundFailed` (`system`, `silent`, or a set's sound: `kelp-failed`). Changed through the app commands `set_sounds` and `set_sound`; `play_sound` plays a sound or a set to hear it. A saved set or sound a later version no longer has is the system's.
- **Played with `rodio`** (Vorbis, and WAV for the system's file), on a thread of its own, opening the default output each time (it may have changed since). Best effort: without a sound device, nothing is heard.

**Alternatives:**
- Two separate pickers only (the first version): more freedom, but most people want a pair that belongs together; Custom keeps the freedom.
- Any sound file on the computer: more choice, but a path to keep valid and a file picker for a small thing; Ottie's own sets sound like Ottie.
- Windows' other toast sounds (IM, Mail, Reminder, SMS…): no files, but Windows only.
- Each OS's own player instead of `rodio` (PlaySound on Windows, `pw-play`/`paplay` on Linux, `afplay` on macOS): no audio crate, but WAV only, more code per OS, and on Linux it depends on tools being installed. `rodio` costs a build dependency on Linux (the ALSA dev package).
- WAV files: nothing to encode, but ten times the size in the app.

**Limits:** on Linux, a notification server that plays its own sound for every notification may play it besides Ottie's, or with Silent; `tauri-plugin-notification` can't ask it not to.
