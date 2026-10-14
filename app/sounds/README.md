# Ottie's notification sounds

Built into the desktop app (`app/src/sound.rs`, `docs/decisions/0038`). Each set has a sound for a finished download (`<set>-finished.ogg`) and one for a failed download (`<set>-failed.ogg`).

All of them are set to the same loudness: 3 dB quieter than Windows' own notification sound (`C:\Windows\Media\Windows Notify System Generic.wav`), measured as ITU-R BS.1770 loudness over the loudest 200 ms, with peaks at most -6 dBFS. A louder file sounds much louder than the system's, since the app plays at its own volume. Format: Ogg Vorbis, 48 kHz, mono (stereo for sounds with room in them).

| Set | Finished | Failed | Source | Licence |
|---|---|---|---|---|
| `marimba` | Marimba C5 → G4 | Marimba C4, twice | Recordings of a real marimba from the [Versilian Community Sample Library](https://github.com/sgossner/VCSL) (Versilian Studios), arranged for Ottie | CC0 1.0 |
| `log` | Two struck wooden bars, up a fifth | The same, down a fifth | Synthesized for Ottie: modes of a wooden bar struck by a burst of noise, slightly detuned, in a small room | MPL-2.0, like Ottie |
| `driftwood` | C4 E4 G4, a slow arpeggio | C4 + G5 together, then E5 | Synthesized for Ottie: a marimba-like chime (a sine with a fast-fading partial at about 4×) | MPL-2.0, like Ottie |
| `shell` | C5 E5 G5, a quick arpeggio | E5 → E4 → B4, a bounce | Synthesized for Ottie, the same chime | MPL-2.0, like Ottie |
| `stream` | C6 → G6, a rising fifth | G4 → C4, a falling fifth | Synthesized for Ottie, the same chime | MPL-2.0, like Ottie |
| `splash` | C4 → C6, two octaves up | G4, then G5 + D6 together | Synthesized for Ottie, the same chime | MPL-2.0, like Ottie |
| `burrow` | C4 → G4, a rising fifth | A5 → A3, two octaves down | Synthesized for Ottie, the same chime | MPL-2.0, like Ottie |
| `tide` | C6 E6 G6 C7, a fast run up | B5 → E5, a falling fifth | Synthesized for Ottie, the same chime | MPL-2.0, like Ottie |
| `holt` | Two bell notes, E6 → C6 | A3 twice, a soft knock | Synthesized for Ottie: a small bell (inharmonic partials), then the chime | MPL-2.0, like Ottie |
| `kelp` | A4 → D5, a rising fourth | E5 C5 A4, falling minor | Synthesized for Ottie, the same chime | MPL-2.0, like Ottie |

To add a set: put its two files here (at the loudness above), add it to `SETS` in `app/src/sound.rs`, name it in `ui/messages/{en,fr}.json` (`sound_set_<id>`) and in `ui/src/lib/sounds.ts`, and add its source and licence to this table (a licence such as CC0 or CC-BY; credit its author here if it asks for that).
