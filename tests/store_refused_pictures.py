#!/usr/bin/env python3
# Cuts short two pictures of the Package Store fixture a scenario copied (tests/fixtures/package-store), so that the
# picture worker refuses them: the app's catalog icon (its directory alone is left) and the app's screenshot (half of
# it). tests/scenarios/z_package_store_refused.qs. Usage: store_refused_pictures.py DATA_ROOT
import os
import sys

assets = os.path.join(sys.argv[1], 'package-store', 'assets', 'packages', 'com.example.fixture')
for name, keep in (('icon.ico', 100), (os.path.join('screens', '01-app.png'), None)):
    path = os.path.join(assets, name)
    with open(path, 'rb') as f:
        data = f.read()
    with open(path, 'wb') as f:
        f.write(data[:keep if keep is not None else len(data) // 2])
