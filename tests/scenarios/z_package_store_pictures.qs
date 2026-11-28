# settings: status_mode=1
# settings: status_size=1
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_OFFLINE=1 GRIMOIRE_BG_DELAY_MS=1500 TZ=UTC
# copy: tests/fixtures/package-store .
# Package Store (2.3, issue #236): the catalog's icons and screenshots are decoded by the sandboxed worker, never by
# the app itself. Nothing has made their pictures at this first look (each job is held back 1.5 s here): the card
# shows the package glyph and the detail page its glyph and its screenshot frame, and each picture that comes draws
# the page again and refreshes only its own place, as a part that fills in by itself: one ui refresh of 57 x 87 px
# at the card's icon, and one of 724 x 843 px at the screenshot, and no other. Seen again, the pictures are in the
# cache: the pages have them with no job.
# The Store's status bar keeps the clock: a minute on (the script's clock, which stands still otherwise, so no
# refresh but the ones counted here), the page is drawn again and only the clock's own place is refreshed, 116 x
# 20 px (the time's last digit, and the date beside it, which moves with that digit's width; TZ=UTC so that the
# digits are the same on every machine), and the icon that comes after it still refreshes its own place alone.
# (Before, the Store did not follow the minute: an icon that came after one had changed was refreshed together
# with the clock, in one box around both.)
clock 1700000000
wait 3600
tap library app.store
wait 400
expect library.store.view is list
expect library.store.list.icons is 0
shot 00-catalog-glyphs
expect display.presents not -1
clock 1700000060
wait 200
expect display.presents is ui 104 29 116 20
expect library.store.list.icons is 0
waitfor library.store.list.icons is 1
wait 200
expect display.presents is ui 404 381 57 87
shot 01-catalog-icon
waitbg
tap library store.package.com.example.fixture
wait 400
expect library.store.view is detail
expect library.store.icon.loaded is 1
expect library.store.shot.loaded is 0
expect library.store.shot.coming is 1
shot 02-detail-screenshot-to-come
expect display.presents not -1
waitfor library.store.shot.loaded is 1
wait 200
expect library.store.shot.coming is 0
expect display.presents is ui 559 1310 724 843
shot 03-detail
waitbg
# Back and in again: both pictures are there at once, and nothing is asked of the worker.
tap library store.back
wait 300
expect library.store.list.icons is 1
tap library store.package.com.example.fixture
wait 400
expect library.store.icon.loaded is 1
expect library.store.shot.loaded is 1
expect bg.busy is 0
tap library store.close
wait 300
expect library.sheet is none
quit
