# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# Passes an app asks for back to back are cleaned once, after the last (issue #307), with Gray run app of
# tests/fixtures/kitpanel (rust/tests/kitfix: `make kit-fixtures`), which fills a square 200 px a side about 200, 200 of
# its body, gray 160 on its odd passes and 96 on its even ones, and asks in each of its first seven passes for another
# 100 ms later. Each pass is refreshed fast in the part it changed, the square alone; what they covered is owed one
# Clean refresh after the pause (150 ms with the pen away) from the last of them, so none comes while the app asks
# (its passes come less than the pause apart) and one comes once it has stopped asking for that long.
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
expect display.presents is fast 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200, fast 100 178 200 200
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
