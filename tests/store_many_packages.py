#!/usr/bin/env python3
# A Package Store catalog of N app packages for a scenario, each with an icon file and a screenshot of its own
# (copies of the fixture package's, tests/fixtures/package-store): index.json and assets/ in FOLDER of the data
# root. FOLDER is package-store, the saved catalog the Store opens with no network
# (tests/scenarios/z_package_store_page_first.qs), unless another is named: package-store-served is where
# tests/store-curl/curl answers from, for a first visit with nothing saved
# (tests/scenarios/z_package_store_first_visit.qs); there the last package's screenshot is left out, so its
# download fails. Usage: store_many_packages.py N [FOLDER] DATA_ROOT
import json
import os
import shutil
import sys

n, root = int(sys.argv[1]), sys.argv[-1]
folder = sys.argv[2] if len(sys.argv) > 3 else 'package-store'
fixture = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'fixtures', 'package-store', 'package-store', 'assets',
                       'packages', 'com.example.fixture')
packages = []
for k in range(n):
    pid = f'com.example.p{k:02d}'
    dest = os.path.join(root, folder, 'assets', 'packages', pid)
    os.makedirs(os.path.join(dest, 'screens'), exist_ok=True)
    for name in ('icon.ico', 'package.yaml', os.path.join('screens', '01-app.png')):
        if not (folder != 'package-store' and k == n - 1 and name.endswith('.png')):
            shutil.copy(os.path.join(fixture, name), os.path.join(dest, name))
    packages.append({
        'id': pid, 'name': f'Package {k:02d}', 'version': '1.0.0', 'type': 'app', 'author': 'Grimoire tests',
        'category': 'Examples', 'summary': 'One of a scenario\'s packages.',
        'description': 'One of a scenario\'s packages.', 'license': 'MIT',
        'manifest': f'packages/{pid}/package.yaml', 'icon': f'packages/{pid}/icon.ico',
        'screenshots': [f'packages/{pid}/screens/01-app.png'], 'tags': ['example'], 'added': '2026-11-22'})
index = {'version': 1, 'name': 'Grimoire Package Store', 'tagline': 'Apps and widgets made for Grimoire.',
         'updated': '2026-11-22', 'featured': 'com.example.p00', 'packages': packages}
with open(os.path.join(root, folder, 'index.json'), 'w', encoding='utf-8') as f:
    json.dump(index, f, indent=2)
