# settings: status_mode=1
# rust-only
# copy: tests/fixtures/apppkg .
# env: GRIMOIRE_BG_DELAY_MS=2500
# Apps (2.3, issue #236): an installed app's bundled icon is decoded by the sandboxed worker, never by the app
# itself. At the first start after an install nothing has made its picture yet: the home's app row shows the app's
# built-in icon, the picture's job is asked for (held back 2.5 s here), and when it has ended the launcher is drawn
# again and only the icon's own place is refreshed, as a part that fills in by itself: a ui refresh of 76 x 85 px at
# the icon. (`has`, not `is`: the launch's repaints, and the home's own redraw when the minute changes, may fall in
# the same stretch, each a refresh of the whole screen; a picture that refreshed the whole screen would leave no
# refresh of the icon's rect.) From then on the picture is in the cache: All apps draws it at once, with no job.
wait 300
expect library.apps is com.example.fixture
expect library.apps.icons is 0
expect bg.busy is 1
shot 00-built-in-icon
expect display.presents not -1
waitfor library.apps.icons is 1
wait 200
expect display.presents has ui 1188 281 76 85
shot 01-the-apps-icon
waitbg
tap library apps
wait 300
expect library.apps.icons is 1
expect bg.busy is 0
shot 02-all-apps
quit
