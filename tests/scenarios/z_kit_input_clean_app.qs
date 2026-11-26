# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# What an app's passes refreshed fast while a finger was down is cleaned after the lift (issue #272), with Gray app of
# tests/fixtures/kitpanel (rust/tests/kitfix: `make kit-fixtures`), which fills a gray (128) square 200 px a side
# about its last touch. A fast refresh shows the gray dithered to black and white, and the lift's refresh drives only
# the pixels that change: so once the contact has lifted, after the editor's pause (150 ms with the pen away), all
# the fast refreshes covered gets one Clean refresh (apphost.rs apphost_wear_tick), and the panel holds the gray. A
# press whose lift changes nothing; a drag that pauses before its lift (its last turn handed over every sample);
# then the panel 30 s later.
clock 1700000000
wait 3600
tap library apps
wait 300
tap library app.test.kit.gray
wait 400
expect library.app.open is 1
waitfor display.stale.0.0.1860.2480 is 0
expect display.presents not -1
touch down 900 1400
wait 700
expect display.presents is fast 100 178 900 1322
expect display.stale.700.1200.400.400 not 0
touch up 900 1400
wait 3000
expect library.app.status has r01
expect display.presents is clean 100 178 900 1322
expect display.stale.0.0.1860.2480 is 0
touch down 300 1000
wait 20
touch move 400 1000
wait 20
touch move 500 1000
wait 20
touch move 600 1000
wait 20
touch move 700 1000
wait 20
touch move 800 1000
wait 20
touch move 900 1000
wait 500
touch up 900 1000
wait 3000
expect library.app.status has e01
expect display.presents is fast 300 900 700 600, fast 300 900 700 200, clean 300 900 700 600
expect display.stale.0.0.1860.2480 is 0
wait 30000
expect display.presents is
expect display.stale.0.0.1860.2480 is 0
quit
