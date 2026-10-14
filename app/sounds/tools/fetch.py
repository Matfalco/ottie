"""Downloads the CC0 sources library.py builds on: three UI sound packs and
recordings of real instruments from the Versilian Community Sample Library.
Each pack goes in a folder of its own; zips are extracted into its `x/`.

Usage: python -I fetch.py <sources-dir>
"""
import sys
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path

PACKS = {
    "lamb": ("https://opengameart.org/sites/default/files/ui_wav.zip", "ui_wav.zip"),
    "haeldb": ("https://opengameart.org/sites/default/files/ui_sounds.zip", "ui_sounds.zip"),
    "kenney": ("https://github.com/Calinou/kenney-interface-sounds/archive/master.zip", "master.zip"),
}

VCSL = "https://raw.githubusercontent.com/sgossner/VCSL/master/Idiophones/"
# (folder in VCSL, local folder, files)
INSTRUMENTS = [
    ("Plucked Idiophones/Kalimba, Tanzania", "kalimba", [
        f"MBira3_pluck_Main_{note}_50_100_rr2.wav"
        for note in ["D4_k19", "F4_k21", "G4_k22", "A#4_k2", "C#5_k24", "E5_k25", "G#5_k26", "C#4_k5"]
    ]),
    ("Plucked Idiophones/Kalimba, Kenya", "kalimba", [
        f"Mbira6_Normal_MainSpirit_{note}_vl3_rr2.wav" for note in ["A4_k1", "B4_k15", "F#4_k14"]
    ]),
    ("Struck Idiophones/Glockenspiel", "glockenspiel", [
        "glock_soft_G4_01.wav", "glock_soft_C5_02.wav", "glock_soft_G5_01.wav",
        "glock_soft_C6_01.wav", "glock_soft_G6_01.wav", "glock_soft_C7_03.wav",
    ]),
    ("Struck Idiophones/Marimba", "marimba", [
        f"Marimba_hit_Outrigger_{note}_soft_01.wav" for note in ["C4", "G4", "B4", "F5", "C6", "F3"]
    ]),
    ("Struck Idiophones/Hand Chimes", "hand-chimes", [
        f"sus_{note}_r01_main.wav" for note in ["A4", "C5", "D5", "E5", "C6", "F#4"]
    ]),
    ("Struck Idiophones/Woodblock", "woodblock", ["wood_click_mp.wav", "wood_click_pp_rr1.wav"]),
    ("Struck Idiophones/Slit Drum", "slit-drum", ["LogDrumHi_MedM_v2_rr1_Sum.wav", "LogDrumLo_MedM_v2_rr1_Sum.wav"]),
    ("Struck Idiophones/Finger Cymbals", "finger-cymbals", ["Fing_Cymb.wav"]),
]


def main():
    root = Path(sys.argv[1])
    for name, (url, file) in PACKS.items():
        folder = root / name
        folder.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(url, folder / file)
        with zipfile.ZipFile(folder / file) as archive:
            archive.extractall(folder / "x")
        print(name)
    for source, local, files in INSTRUMENTS:
        folder = root / "vcsl" / local
        folder.mkdir(parents=True, exist_ok=True)
        for file in files:
            urllib.request.urlretrieve(VCSL + urllib.parse.quote(f"{source}/{file}"), folder / file)
        print(f"vcsl/{local}")


if __name__ == "__main__":
    main()
