# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# A pass an app asks for refreshes what it changed, and is cleaned after (issue #307), with Gray tick app of
# tests/fixtures/kitpanel (rust/tests/kitfix: `make kit-fixtures`), which fills a square 200 px a side about 200, 200 of
# its body, gray 160 on its odd passes and 96 on its even ones, and asks in each pass for another a second later. Each
# pass is refreshed fast in the part it changed, the square alone (it was the whole screen). A fast refresh shows the
# gray dithered to black and white, and a later refresh drives only the pixels that change: so after the pause (150 ms
# with the pen away) all the fast refreshes covered gets one Clean refresh (apphost.rs apphost_wear_tick), and the panel
# holds the gray. (The status bar's clock held still: its minute would bring a pass of its own.)
clock 1700000000
wait 3600
tap library apps
wait 300
tap library app.test.kit.graytick
wait 400
expect library.app.open is 1
expect library.app.status has p001
waitfor display.stale.0.0.1860.2480 is 0
expect display.presents not -1
# the pass a second after the launch's: the square, fast, its gray dithered
waitfor library.app.status has p002
expect display.presents is fast 100 178 200 200
expect display.stale.100.178.200.200 not 0
# no clean before the pause, one of the square after it, and the panel holds the gray
wait 120
expect display.presents is
wait 60
expect display.presents is clean 100 178 200 200
expect display.stale.0.0.1860.2480 is 0
# the next pass the same, a second after the last
wait 700
expect library.app.status has p002
expect display.presents is
wait 200
expect library.app.status has p003
expect display.presents is fast 100 178 200 200
expect display.stale.100.178.200.200 not 0
wait 300
expect display.presents is clean 100 178 200 200
expect display.stale.0.0.1860.2480 is 0
shot 01-graytick
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
quit
