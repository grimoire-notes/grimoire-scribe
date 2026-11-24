#!/bin/sh
# Runs tools/kindle-dev headlessly against a scratch mount and inspects the launcher it patches
# out of the zip: the dev copy's name and root, GRIMOIRE_DEV (which turns updates off in the
# copy), the app archive intact behind the patch, and the --name/--root refusals that keep it
# off the release. A launcher change that breaks the patch fails here, before anything reaches
# a Kindle.
# usage: tests/kindle_dev_check.sh [build/dist/Grimoire.zip]
set -u
cd "$(dirname "$0")/.." || exit 1
REPO=$(pwd)
ZIP=${1:-build/dist/Grimoire.zip}
case "$ZIP" in /*) ;; *) ZIP=$REPO/$ZIP ;; esac
if [ ! -f "$ZIP" ]; then
    echo "kindle dev check: $ZIP is not there (make package)"
    exit 1
fi
unset GRIMOIRE_MTP_MOUNT
W=$(cd "$(dirname "$ZIP")/.." && pwd)/kindle-dev-check
MNT=$W/mnt
rm -rf "$W" && mkdir -p "$MNT/documents"
fail=0
check() {  # check NAME COMMAND...: ok when the command succeeds
    name=$1
    shift
    if "$@"; then echo "ok   $name"; else echo "FAIL $name"; fail=1; fi
}
refuse() {  # refuse DESC ARGS...: kindle-dev must refuse the run
    desc=$1
    shift
    if "$REPO/tools/kindle-dev" --zip "$ZIP" "$@" >"$W/refuse.log" 2>&1; then
        echo "FAIL $desc (it went through)"
        fail=1
    else
        echo "ok   $desc"
    fi
}
DEV=$MNT/documents/Grimoire\ \(dev\).sh

# The default install into a scratch mount (no Kindle, no MTP: --mount points at the folder).
if "$REPO/tools/kindle-dev" --mount "$MNT" --zip "$ZIP" >"$W/install.log" 2>&1; then
    echo "ok   installs into a scratch mount"
else
    echo "FAIL installs into a scratch mount"
    cat "$W/install.log"
    fail=1
fi
check "the dev launcher is documents/Grimoire (dev).sh" test -f "$DEV"

if [ -f "$DEV" ]; then
    check "the Name key stays in SH_Integration's first six lines" \
        sh -c 'head -n 6 "$1" | grep -qxF "# Name: Grimoire (dev)"' sh "$DEV"
    check "GRIMOIRE_US defaults to the dev root, overridable with GRIMOIRE_DEV_US" \
        grep -qxF 'GRIMOIRE_US=${GRIMOIRE_DEV_US:-/mnt/us/grimoire-dev}' "$DEV"
    check "GRIMOIRE_DEV=1, which kindle/app/grimoire.sh reads to turn updates off" \
        grep -qxF 'GRIMOIRE_DEV=1' "$DEV"
    check "GRIMOIRE_US and GRIMOIRE_DEV reach the app" \
        grep -qxF 'export GRIMOIRE_US GRIMOIRE_DEV' "$DEV"
    check "the app archive is still behind the patch" grep -qxF '__GRIMOIRE_APP_ARCHIVE__' "$DEV"
    # Only the name line and the three dev lines may differ from the release launcher; its head
    # and its appended archive are otherwise the zip's own bytes.
    check "the patch changes nothing but the name and the dev lines" python3 -I - "$ZIP" "$DEV" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as archive:
    rel = archive.read("documents/Grimoire.sh")
dev = open(sys.argv[2], "rb").read()
marker = b"\n__GRIMOIRE_APP_ARCHIVE__\n"
rel_head, _, rel_tail = rel.partition(marker)
dev_head, _, dev_tail = dev.partition(marker)
insert = (
    b"GRIMOIRE_US=${GRIMOIRE_DEV_US:-/mnt/us/grimoire-dev}\n"
    b"GRIMOIRE_DEV=1\n"
    b"export GRIMOIRE_US GRIMOIRE_DEV\n"
)
back = dev_head.replace(b"# Name: Grimoire (dev)\n", b"# Name: Grimoire\n", 1)
back = back.replace(insert, b"", 1)
if not dev_tail or dev_tail != rel_tail or back != rel_head:
    raise SystemExit("the patched launcher differs from the release's beyond the name and the dev lines")
PY
    # The part before the appended archive is what a shell parses (kindle-dev checks it too).
    python3 -I - "$DEV" "$W/head.sh" <<'PY'
import sys

data = open(sys.argv[1], "rb").read()
head = data.partition(b"\n__GRIMOIRE_APP_ARCHIVE__\n")[0]
open(sys.argv[2], "wb").write(head + b"\n")
PY
    check "the patched launcher's head parses" sh -n "$W/head.sh"
fi

# --prod updates the release launcher in place: the zip's own documents/Grimoire.sh, byte for
# byte (no name, no dev root, no GRIMOIRE_DEV), because a dev root there would hide the release's
# notebooks. A zip of another version than this branch's VERSION is refused.
if "$REPO/tools/kindle-dev" --mount "$MNT" --zip "$ZIP" --prod >"$W/prod.log" 2>&1; then
    echo "ok   --prod installs the release launcher"
else
    echo "FAIL --prod installs the release launcher"
    cat "$W/prod.log"
    fail=1
fi
check "--prod writes documents/Grimoire.sh" test -f "$MNT/documents/Grimoire.sh"
check "--prod installs the zip's launcher unchanged" python3 -I - "$ZIP" "$MNT/documents/Grimoire.sh" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as archive:
    release = archive.read("documents/Grimoire.sh")
if open(sys.argv[2], "rb").read() != release:
    raise SystemExit("documents/Grimoire.sh is not the zip's own launcher")
PY
python3 -I - "$ZIP" "$W/other-version.zip" <<'PY'
import sys
import zipfile

src, dst = sys.argv[1:3]
with zipfile.ZipFile(src) as zin, zipfile.ZipFile(dst, "w") as zout:
    for item in zin.infolist():
        data = zin.read(item.filename)
        if item.filename == "README.txt":
            rest = data.split(b"\n", 1)[1]
            data = b"Grimoire 0.0.0-other: another version\n" + rest
        zout.writestr(item, data)
PY
if "$REPO/tools/kindle-dev" --mount "$MNT" --zip "$W/other-version.zip" --prod >"$W/prod-refuse.log" 2>&1; then
    echo "FAIL --prod refuses a zip of another version (it went through)"
    fail=1
else
    echo "ok   --prod refuses a zip of another version"
fi
check "the refused zip left the release launcher alone" python3 -I - "$ZIP" "$MNT/documents/Grimoire.sh" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as archive:
    release = archive.read("documents/Grimoire.sh")
if open(sys.argv[2], "rb").read() != release:
    raise SystemExit("a refused zip replaced the release launcher")
PY
# --prod is the release, not a copy of it: a name or a root would make it one.
refuse "--prod refuses --name" --prod --mount "$MNT" --name 'Grimoire (dev)'
refuse "--prod refuses --root" --prod --mount "$MNT" --root /mnt/us/grimoire-dev

# A --name and --root of its own land in the header.
if "$REPO/tools/kindle-dev" --mount "$MNT" --zip "$ZIP" --name 'Grimoire (test)' \
    --root /mnt/us/grimoire-test >"$W/custom.log" 2>&1; then
    echo "ok   accepts a --name and --root of its own"
else
    echo "FAIL accepts a --name and --root of its own"
    cat "$W/custom.log"
    fail=1
fi
check "--name lands in the header" grep -qxF '# Name: Grimoire (test)' "$MNT/documents/Grimoire (test).sh"
check "--root lands in the header" \
    grep -qxF 'GRIMOIRE_US=${GRIMOIRE_DEV_US:-/mnt/us/grimoire-test}' "$MNT/documents/Grimoire (test).sh"

# Nothing that could write over the release gets through.
refuse "refuses --name Grimoire" --name Grimoire
refuse "refuses --name grimoire - reset pen calibration, any spelling" --name "GRIMOIRE - reset pen CALIBRATION"
refuse "refuses --root /mnt/us (not strictly under it)" --root /mnt/us
refuse "refuses --root with a .. segment" --root /mnt/us/grimoire-dev/..
refuse "refuses --root outside /mnt/us" --root /tmp
refuse "refuses --root /mnt/us2 (not under /mnt/us/)" --root /mnt/us2

if [ "$fail" -ne 0 ]; then
    echo "KINDLE DEV CHECK FAILED"
    exit 1
fi
echo "KINDLE DEV CHECK PASSED"
