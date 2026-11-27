#!/usr/bin/env python3
# Plants the tree tests/scenarios/files_43_places_named.qs browses, in the storage root a "# storage:" scenario runs
# on: a storage that names Files' places otherwise (issue #278: Document, Download), with folders of the Kindle's
# names that are no place there (documents, fonts), a system folder of its own (Stock) and a top folder that is no
# place (Notes). Usage: files_places_named_tree.py STORAGE_ROOT
import os
import sys

root = sys.argv[1]
for d in ['grimoire/import', 'grimoire/trash', 'Document/Books', 'Download', 'documents', 'fonts', 'Stock/Inside', 'Notes']:
    os.makedirs(os.path.join(root, d), exist_ok=True)
for f, data in [('Document/Books/Book.pdf', b'%PDF-1.4\n'), ('Download/Daily.pdf', b'%PDF-1.4\n'), ('fonts/Kindle Serif.ttf', b'x')]:
    with open(os.path.join(root, f), 'wb') as fh:
        fh.write(data)
