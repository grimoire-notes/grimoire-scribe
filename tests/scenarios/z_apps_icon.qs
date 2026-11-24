# settings: status_mode=1
# rust-only
# copy: tests/fixtures/apppkg .
# env: GRIMOIRE_BG_DELAY_MS=8000
# Apps (2.3, issue #236): an installed app's bundled icon is decoded by the sandboxed worker, never by the app
# itself. At the first start after an install nothing has made its picture yet: the home's app row shows the app's
# built-in icon, the picture's job is asked for (held back 8 s here, well past the launch's repaints), and when it has
# ended the launcher is drawn again and only the icon's own place is refreshed, as a part that fills in by itself
# (one ui refresh of the icon, no whole screen). From then on the picture is in the cache: All apps and the package
# manager's list draw it at once, with no job.
clock 1700000000
wait 3600
expect library.apps is com.example.fixture
expect library.apps.icons is 0
expect bg.busy is 1
shot 00-built-in-icon
expect display.presents not -1
waitfor library.apps.icons is 1
wait 200
expect display.presents is ui 1188 281 76 85
shot 01-the-apps-icon
waitbg
tap library apps
wait 300
expect library.apps.icons is 1
expect bg.busy is 0
shot 02-all-apps
quit
