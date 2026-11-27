# settings: status_mode=1
# rust-only
# copy: tests/fixtures/apppkg .
# env: GRIMOIRE_BG_DELAY_MS=5000
# Apps (2.3, issue #236): an installed app's bundled icon is decoded by the sandboxed worker, never by the app
# itself. At the first start after an install nothing has made its picture yet: the home's app row shows the app's
# built-in icon, the picture's job is asked for (held back 5 s here, past the launch's two repaints), and when it
# has ended the launcher is drawn again and only the icon's own place is refreshed, as a part that fills in by
# itself: one ui refresh of 104 x 116 px at the icon, the row's fourth square, after Files, Package Store and Share
# (#319; the icon drawn at twice its size in the middle of its square since #332), and no other (the script's clock
# stands still, so the home does not draw itself again for a new minute in the stretch). From then on the picture is
# in the cache: All apps draws it at once, with no job.
clock 1700000000
wait 300
expect library.apps is com.example.fixture
expect library.apps.icons is 0
expect bg.busy is 1
shot 00-built-in-icon
wait 3300
expect library.apps.icons is 0
expect display.presents not -1
waitfor library.apps.icons is 1
wait 200
expect display.presents is ui 765 306 104 116
shot 01-the-apps-icon
waitbg
tap library apps
wait 300
expect library.apps.icons is 1
expect bg.busy is 0
shot 02-all-apps
quit
