# settings: status_mode=1
# settings: status_size=1
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_OFFLINE=1 GRIMOIRE_BG_DELAY_MS=600
# copy: tests/fixtures/package-store .
# generate: tests/store_refused_pictures.py
# Package Store (2.3, issue #236): pictures the sandboxed worker refuses (the app's icon and its screenshot, each
# cut short here). The card keeps the package glyph, the detail page its glyph, and the screenshot's frame its glyph
# without the loading words; a good picture beside them (the widget's screenshot) still shows. Each refused file is
# marked, so seen again nothing is asked of the worker (a job would still be held back here, 600 ms).
# The frame says a screenshot is loading while its job is on, and is drawn again when the job has ended with no
# picture: the one refresh of that stretch is the words' own place, 408 x 36 px, as they go (the icon's job, which
# ends with nothing too, changes nothing on the page and refreshes nothing; the script's clock stands still).
clock 1700000000
wait 3600
tap library app.store
wait 400
expect library.store.view is list
waitbg
wait 200
expect library.store.list.icons is 0
tap library store.package.com.example.fixture
wait 200
expect library.store.view is detail
expect library.store.shot.coming is 1
expect display.presents not -1
waitbg
wait 200
expect library.store.icon.loaded is 0
expect library.store.shot.loaded is 0
expect library.store.shot.coming is 0
expect display.presents is ui 724 1784 408 36
shot 00-refused-pictures
tap library store.back
wait 300
tap library store.package.com.example.timer
wait 300
waitfor library.store.shot.loaded is 1
shot 01-a-good-screenshot
waitbg
tap library store.back
wait 300
expect library.store.list.icons is 0
tap library store.package.com.example.fixture
wait 300
expect library.store.view is detail
expect bg.busy is 0
expect library.store.icon.loaded is 0
expect library.store.shot.loaded is 0
expect library.store.shot.coming is 0
tap library store.close
wait 300
expect library.sheet is none
quit
