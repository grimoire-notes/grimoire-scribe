# settings: status_mode=1
# settings: status_size=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# Passes an app asks for back to back are refreshed fast and cleaned once, after the last (issue #307), with Gray run app
# of tests/fixtures/kitpanel (rust/tests/kitfix: `make kit-fixtures`), which fills a square 200 px a side about 200, 200
# of its body, gray 160 on its odd passes and 96 on its even ones, and asks in each of its first seven passes for
# another 100 ms later. Each pass refreshes the square alone. The first comes after a rest (no pass asked for and no
# fast refresh within the pause, 150 ms with the pen away), so it is refreshed as the UI is; each later one comes back
# to back, less than the pause after the last, so it is refreshed fast, and what they covered is owed one Clean refresh
# after the pause from the last of them: none comes while the app asks, and one once it has stopped for that long.
clock 1700000000
wait 3600
tap library apps
wait 300
tap library app.test.kit.grayrun
wait 50
expect library.app.open is 1
expect library.app.status has p001
expect display.presents not -1
waitfor library.app.status has p008
expect display.presents is ui 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200
expect display.stale.100.178.200.200 not 0
wait 120
expect library.app.status has p008
expect display.presents is
wait 60
expect display.presents is clean 100 178 200 200
expect display.stale.0.0.1860.2480 is 0
wait 3000
expect library.app.status has p008
expect display.presents is
expect display.stale.0.0.1860.2480 is 0
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
quit
