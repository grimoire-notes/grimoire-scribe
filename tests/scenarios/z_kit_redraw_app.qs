# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# A pass an app asks for after a rest refreshes what it changed at full depth (issue #307), with Gray tick app of
# tests/fixtures/kitpanel (rust/tests/kitfix: `make kit-fixtures`), which fills a square 200 px a side about 200, 200 of
# its body, gray 160 on its odd passes and 96 on its even ones, and asks in each pass for another a second later. A
# second is more than the pause (150 ms with the pen away) since its last pass, with no fast refresh owed a clean: so
# each pass is refreshed as the UI is (apphost.rs apphost_tick), the square alone (it was a fast refresh of the whole
# screen), and the panel holds the gray at once, with no clean after it. (The status bar's clock held still: its minute
# would bring a pass of its own.)
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
# the pass a second after the launch's: the square at full depth, the gray on the panel at once
waitfor library.app.status has p002
expect display.presents is ui 100 178 200 200
expect display.stale.0.0.1860.2480 is 0
# no clean after it, and no pass before the next second
wait 300
expect display.presents is
wait 600
expect library.app.status has p002
expect display.presents is
wait 200
expect library.app.status has p003
expect display.presents is ui 100 178 200 200
expect display.stale.0.0.1860.2480 is 0
# and so on, one refresh of the square a second, nothing else: each pass in a wait of its own, as the one before
# (a wait's time is made up in one step at its end on a slow machine, docs/SIM.md: one wait that held two passes
# gave the second, asked for by the first, no time to come due). The third pass ran in the last 100 ms, so the
# fourth is due 900 to 1,000 ms from here and the fifth 1,900 to 2,100 ms.
wait 800
expect library.app.status has p003
expect display.presents is
wait 300
expect library.app.status has p004
expect display.presents is ui 100 178 200 200
expect display.stale.0.0.1860.2480 is 0
wait 700
expect library.app.status has p004
expect display.presents is
wait 400
expect library.app.status has p005
expect display.presents is ui 100 178 200 200
expect display.stale.0.0.1860.2480 is 0
shot 01-graytick
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
quit
