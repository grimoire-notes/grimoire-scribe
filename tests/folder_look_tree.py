#!/usr/bin/env python3
# Plants the tree the folder look scenario (tests/scenarios/y_folder_look.qs) starts from, in the storage root a
# "# storage:" scenario runs on: an empty Library folder, School, and one photo in grimoire/photos/. The photo's
# preview is made by the app's own background job; the scenario waits for it.
# Usage: folder_look_tree.py STORAGE_ROOT
import os
import shutil
import sys

root = sys.argv[1]
g = os.path.join(root, 'grimoire')
here = os.path.dirname(os.path.abspath(__file__))

os.makedirs(os.path.join(g, 'photos'), exist_ok=True)
shutil.copyfile(os.path.join(here, 'fixtures/photos/IMG_0001.jpg'), os.path.join(g, 'photos/cover.jpg'))
with open(os.path.join(g, 'folders.ini'), 'w', encoding='utf-8') as f:
    f.write('# Grimoire folders\nSchool\n')
