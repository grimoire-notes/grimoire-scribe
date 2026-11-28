#!/usr/bin/env python3
"""The theme fonts: makes the faces Cassette draws with (rust/src/looks/), each subset as
tools/fonts_classic.py subsets the Classic families, with its OFL text beside it.

For the maintainer, through `make fonts-subset` (assets/README.md), after the Classic fonts. It needs
fontTools 4.68.0, as fonts_classic.py does (its FONTTOOLS); the gate never runs it.

  1. Downloads, once, into the cache folder: each font and its family's OFL text from its repository
     at a pinned commit (each years old: the project's two-week rule), and checks each by SHA-256.
     The fonts are static already: nothing is instanced.
  2. Subsets each to fonts_classic.py's UNICODES (Basic Latin, Latin-1, Latin Extended-A, General
     Punctuation, the euro, the arrows, the minus sign, the replacement character) and the few marks a
     theme draws beyond them (EXTRA), with fonts_classic.py's options: kerning kept, hinting, glyph
     names, substitutions and vertical metrics gone, the names 0 to 6, 13 and 14 kept. What a font
     lacks, Terminess draws (rust/src/font.rs's fallback), as for every face.
  3. Checks the result: every character of UNICODES and EXTRA the source has is still there, the font
     is no larger than its source, and the family name is the source's. A family whose OFL text has a
     Reserved Font Name is refused, as fonts_classic.py refuses one: a subset is a Modified Version,
     which may not carry that name.

With --verify it writes nothing: it makes every font in memory and fails unless each matches the file
already in the output folder byte for byte.
"""
import argparse
import io
import os
import shutil
import subprocess
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from fonts_classic import FONTTOOLS, NAME_IDS, UNICODES, sha256, write  # noqa: E402

GF = "google/fonts"
# Each font: the file written here, its repository, the commit and path of the file shipped (the
# newest commit that touched it), its SHA-256, and its family's OFL text there with its SHA-256.
FONTS = [
    # Cassette (looks/cassette.rs): JetBrains Mono for text, Space Mono for caps, Archivo Black for titles
    ("JetBrainsMono-Regular.ttf", "JetBrains/JetBrainsMono", "02bb50b082dad9ef8a0f33ac393839202b760223",
     "fonts/ttf/JetBrainsMono-Regular.ttf", "e6fd0d7e91550b3ed2b735d4312474362c4716edc4fc0577a0f61ed782d5aed1"),
    ("JetBrainsMono-Bold.ttf", "JetBrains/JetBrainsMono", "02bb50b082dad9ef8a0f33ac393839202b760223",
     "fonts/ttf/JetBrainsMono-Bold.ttf", "d22c4f3821d725eb01210d278d95dfcfcaadc34699a06658d47c8a5cc5830ada"),
    ("SpaceMono-Regular.ttf", GF, "fa55d0564c2bc8e1e308be751a699673a723b9a9",
     "ofl/spacemono/SpaceMono-Regular.ttf", "95837e182baeeada83368f7748db28357f0a1b75c6b84ff7065b5edf933c8e18"),
    ("SpaceMono-Bold.ttf", GF, "fa55d0564c2bc8e1e308be751a699673a723b9a9",
     "ofl/spacemono/SpaceMono-Bold.ttf", "405e73d41afb7e5906efce206a326af5c956f38e255f35421c260e861e599c59"),
    ("ArchivoBlack-Regular.ttf", GF, "e723d317895c9dd66f7f87012577bab2c0ea6b5d",
     "ofl/archivoblack/ArchivoBlack-Regular.ttf", "dd9a89a019b4849f66ab75455fe7bdf931311042cbb0f0f97acc061539703180"),
]
# Each family's OFL text: written as <stem>-OFL-1.1.txt (its stem begins each of its fonts' file names,
# which tests/package_check.sh pairs them by), from its repository at a pinned commit.
OFLS = [
    # (google/fonts' copy: the same license as the font repository's, its text laid out as the others')
    ("JetBrainsMono", GF, "2e05c1cf00a6e4f40a4b931600a90881c26e15cd", "ofl/jetbrainsmono/OFL.txt",
     "b2fe5e8987594e9ffd1d2ca52a2f5d73eb8335243893c5d6254b5ad69269591d"),
    ("SpaceMono", GF, "fa55d0564c2bc8e1e308be751a699673a723b9a9", "ofl/spacemono/OFL.txt",
     "8e4ee42b2553e1e01504e61cb0d46d148cd8c9e5eacaa3622a7df2d4f2955b9f"),
    ("ArchivoBlack", GF, "e723d317895c9dd66f7f87012577bab2c0ea6b5d", "ofl/archivoblack/OFL.txt",
     "3173acd82f8c6159b5b1037b539fcbd4edff68e65c2ea8b9412b5a5ca97b08ff"),
]
# What the themes draw beyond UNICODES: Cassette's paper heading's triangle (U+25B8).
EXTRA = [0x25B8]


def fail(msg):
    sys.exit("fonts-themes: " + msg)


def fetch(cache, repo, commit, path, want):
    """A file of a repository at a commit: downloaded into the cache once, checked by SHA-256."""
    os.makedirs(cache, exist_ok=True)
    local = os.path.join(cache, "%s-%s-%s" % (repo.replace("/", "_"), commit[:12], path.replace("/", "_")))
    url = "https://raw.githubusercontent.com/%s/%s/%s" % (repo, commit, path)
    if not os.path.exists(local):
        part = local + ".part"
        print("fonts-themes: downloading %s" % url)
        if shutil.which("curl"):
            subprocess.run(["curl", "-fsSL", "--retry", "3", "-o", part, url], check=True)
        else:
            with urllib.request.urlopen(url) as r, open(part, "wb") as out:
                shutil.copyfileobj(r, out)
        os.replace(part, local)
    with open(local, "rb") as f:
        data = f.read()
    if sha256(data) != want:
        fail("%s is not the expected download (SHA-256 %s, expected %s); delete it to fetch it again"
             % (local, sha256(data), want))
    return data


def make(src):
    from fontTools import subset
    from fontTools.ttLib import TTFont
    font = TTFont(io.BytesIO(src), recalcBBoxes=False, recalcTimestamp=False)
    opts = subset.Options()
    opts.layout_features = ["kern"]
    opts.name_IDs = NAME_IDS
    opts.notdef_outline = True
    opts.hinting = False
    opts.glyph_names = False
    opts.recalc_timestamp = False
    opts.drop_tables += ["STAT", "GSUB", "vhea", "vmtx"]
    sub = subset.Subsetter(options=opts)
    sub.populate(unicodes=UNICODES + EXTRA)
    sub.subset(font)
    out = io.BytesIO()
    font.save(out)
    return out.getvalue()


def check_result(src, data, label):
    from fontTools.ttLib import TTFont
    was, got = TTFont(io.BytesIO(src)), TTFont(io.BytesIO(data))
    had, has = was.getBestCmap(), got.getBestCmap()
    lost = [cp for cp in UNICODES + EXTRA if cp in had and cp not in has]
    if lost:
        fail("%s: lost %s" % (label, ["U+%04X" % c for c in lost]))
    if len(data) >= len(src):
        fail("%s: %d bytes, no smaller than its source's %d" % (label, len(data), len(src)))
    family = got["name"].getDebugName(1)
    want = was["name"].getDebugName(1)
    if family != want:
        fail("%s: family name %r, not %r" % (label, family, want))
    lacks = [cp for cp in EXTRA if cp not in has]
    if lacks:
        print("fonts-themes: %s: the source has no %s (Terminess draws it)" % (label, " ".join("U+%04X" % c for c in lacks)))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--cache", required=True, help="download folder, outside the repo")
    ap.add_argument("--out", required=True, help="where the fonts and their OFL texts go (assets)")
    ap.add_argument("--verify", action="store_true", help="write nothing; compare with the files in --out")
    args = ap.parse_args()
    try:
        import fontTools
    except ImportError:
        fail("needs fontTools: python3 -m pip install fonttools==%s (in a venv), then "
             "make fonts-subset FONTS_PYTHON=<that venv>/bin/python" % FONTTOOLS)
    if fontTools.version != FONTTOOLS:
        print("fonts-themes: warning: the committed fonts were made with fontTools %s; this one (%s) may "
              "write other bytes" % (FONTTOOLS, fontTools.version))
    made = {}
    for stem, repo, commit, path, want in OFLS:
        ofl = fetch(args.cache, repo, commit, path, want)
        head = ofl.split(b"This Font Software is licensed", 1)[0]
        if b"SIL Open Font License, Version 1.1" not in ofl:
            fail("%s's OFL text is not the OFL 1.1" % stem)
        if b"Reserved Font Name" in head:
            fail("%s's OFL text has a Reserved Font Name, which a subset may not carry" % stem)
        made["%s-OFL-1.1.txt" % stem] = ofl
    total = before = 0
    for file_name, repo, commit, path, want in FONTS:
        src = fetch(args.cache, repo, commit, path, want)
        data = make(src)
        check_result(src, data, file_name)
        made[file_name] = data
        total += len(data)
        before += len(src)
    print("fonts-themes: %d fonts, %d bytes in all (%d before the subset)" % (len(FONTS), total, before))
    if args.verify:
        for name, data in made.items():
            path = os.path.join(args.out, name)
            with open(path, "rb") as f:
                if f.read() != data:
                    fail("%s is not what the recipe makes" % path)
            print("fonts-themes: %s matches (%d bytes)" % (path, len(data)))
        return
    for name, data in made.items():
        write(args.out, name, data)


if __name__ == "__main__":
    main()
