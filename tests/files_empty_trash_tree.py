#!/usr/bin/env python3
# Plants the tree the Empty trash scenarios (tests/scenarios/files_44_empty_trash.qs and the fit runs after it, issue
# #330) work on, in the storage root a "# storage:" scenario runs on: three notebooks (Alpha with two pages and Beta at
# the Library's top, Gamma in School), each page an empty page file the editor opens; a photo and a template of known
# sizes; and an empty trash. The scenario deletes into the trash itself (Beta from the Library, Alpha's second page in
# the Pages view, the photo in Files). Usage: files_empty_trash_tree.py STORAGE_ROOT
import os
import struct
import sys

root = sys.argv[1]
g = os.path.join(root, 'grimoire')
T = 1790000000  # 2026-09-21


def notebook(n, name, folder, pages):
    d = os.path.join(g, 'notebooks', f'20261101-000000-000{n}')
    os.makedirs(d, exist_ok=True)
    ids = ','.join(str(i) for i in range(1, pages + 1))
    with open(os.path.join(d, 'notebook.ini'), 'w') as f:
        f.write(f'name={name}\nfolder={folder}\npaper=0\ncreated={T}\nmodified={T + n}\nlast_page=0\n'
                f'next_page_id={pages + 1}\npage_w=1860\npage_h=2480\npages={ids}\n')
    for i in range(1, pages + 1):
        # (an empty page as the store writes one: its header alone)
        with open(os.path.join(d, f'p{i}.qpg'), 'wb') as f:
            f.write(struct.pack('<4sHHIII', b'QPG1', 1, 4, 1860, 2480, 0))


def put(path, size, age):
    p = os.path.join(root, path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'wb') as f:
        f.write(b'x' * size)
    os.utime(p, (T - age, T - age))


notebook(1, 'Alpha', '', 2)
notebook(2, 'Beta', '', 1)
notebook(3, 'Gamma', 'School', 1)
for d in ['import', 'templates', 'photos', 'trash']:
    os.makedirs(os.path.join(g, d), exist_ok=True)
put('grimoire/templates/Grid.png', 1024, 300)
put('grimoire/photos/beach.jpg', 3072, 100)
