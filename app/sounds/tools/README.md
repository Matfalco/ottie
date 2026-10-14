# Making Ottie's notification sounds

The scripts that made `app/sounds/` (`docs/decisions/0038`): they generate or gather candidate sounds, set them all to one loudness, let someone choose by ear in a small local page, then export the chosen pairs as Ogg Vorbis. They're tools for the maintainer, not part of the build.

Needs Python 3 with `numpy`, and `soundfile` for `export.py`. Use a virtual environment outside the repo, and a work folder outside it too (the sources and candidates are large):

```bash
python -m venv <venv> && <venv>/bin/pip install numpy soundfile   # <venv>\Scripts\ on Windows
```

The loudness reference is Windows' notification sound (`C:\Windows\Media\Windows Notify System Generic.wav`), so the scripts that measure against it run on Windows.

| Script | What it does |
|---|---|
| `loudness.py` | ITU-R BS.1770 loudness (K-weighting, loudest window) and peak of WAV files; used by the others. `python -I loudness.py <file.wav>…` |
| `chimes.py` | The first round: marimba-like chimes (a sine with a fast-fading partial at ~4×), deep, middle, high and mixed. Writes WAVs and a listening page. `python -I chimes.py <work>/candidates -3` |
| `pops.py` | The second round: pops, bubbles, drops, bells, plucks, blips. Same output. `python -I pops.py <work>/pops -3` |
| `fetch.py` | Downloads the CC0 sources: Kenney's Interface Sounds, Robin Lamb's and HaelDB's UI sounds (OpenGameArt), and real instrument recordings from the Versilian Community Sample Library. `python -I fetch.py <work>/sources` |
| `library.py` | The picker's library: those recordings, two-note sounds made from the instruments (kalimba, glockenspiel, marimba, hand chimes…), and struck wood, glass and water drops synthesized with a little room reverb. Every sound is set 3 dB quieter than Windows' (loudest 200 ms, peaks at most -6 dBFS). `python -I library.py <work>/sources <work>/picker -3` |
| `add_rounds.py` | Adds the two synthesized rounds to the picker, at the same loudness. `python -I add_rounds.py <work>` |
| `picker/server.py` | The picker, on http://127.0.0.1:8766/: `sort.html` to sort every sound into Yes, Maybe or No as a success or failure sound, then `index.html` to build pairs (a success sound and its failure sound). Choices are saved to `<work>/picker/choices.json` as you go. `python -I picker/server.py <work>/picker` |
| `export.py` | Writes the complete pairs, in order, as `<set>-finished.ogg` / `<set>-failed.ogg`, checking their loudness after encoding. `python -I export.py <work>/picker app/sounds marimba,log,…` |

Then name each set in `app/src/sound.rs` (`SETS`), `ui/messages/{en,fr}.json` and `ui/src/lib/sounds.ts`, and give its source and licence in `app/sounds/README.md`.
