# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script
# copy: tests/fixtures/apphost .
# An app's redraw requests (issue #247), with the packages of tests/fixtures/apphost (written by
# rust/tests/appfix: `make app-fixtures`). App time is the script's (GRIMOIRE_SIM_CLOCK=script): each `wait`
# moves it on by exactly its milliseconds, so what comes due in a stretch is the same on any machine. A pass
# refreshes only the part of the screen it changed (issue #307), and each of these apps draws the same picture in
# every pass (a black rectangle; the count is in its status line alone): so its passes, counted, refresh nothing.
# tests/scenarios/z_kit_redraw_app.qs and z_kit_redraw_run.qs hold the refreshes of passes that change.
# (the status bar's clock held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
expect library.apps is test.app.asks test.app.counts test.app.later test.app.loop test.app.once test.app.ticker test.app.wide
# Ticker counts its drawing passes in its status line and asks, in each, to be drawn again a second later. An
# app whose request is due is drawn again, and the part of the screen the pass changed refreshed, none here; while
# nothing is due, nothing is drawn. (Each `waitfor` ends in the round the pass it names ran in, so the next pass is
# due one second of app time after it.)
tap library apps
wait 300
tap library app.test.app.ticker
wait 400
expect library.app.open is 1
expect library.app.error is
expect library.app.status is 000001
waitfor library.app.status is 000002
expect display.presents not -1
# 900 ms after a pass: no pass.
wait 900
expect library.app.status is 000002
expect display.presents is
# 200 ms more: the pass that was due, which changed nothing on the screen, so no refresh.
wait 200
expect library.app.status is 000003
expect display.presents is
# And so on, a pass a second.
waitfor library.app.status is 000004
expect display.presents is
wait 900
expect library.app.status is 000004
expect display.presents is
wait 200
expect library.app.status is 000005
expect display.presents is
shot 01-ticker
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
# Asks asks, in each pass, to be drawn again at once. The shortest delay is 100 ms: 90 ms after a pass there is
# no pass, and 20 ms later the one pass that was due. So an app that asks as fast as it may is drawn, and the
# screen refreshed, at most ten times a second.
tap library apps
wait 300
tap library app.test.app.asks
wait 400
expect library.app.open is 1
waitfor library.app.status is 000006
expect display.presents not -1
wait 90
expect library.app.status is 000006
expect display.presents is
wait 20
expect library.app.status is 000007
expect display.presents is
waitfor library.app.status is 000008
expect display.presents is
wait 90
expect library.app.status is 000008
expect display.presents is
wait 20
expect library.app.status is 000009
expect display.presents is
waitfor library.app.status is 000010
expect display.presents is
wait 90
expect library.app.status is 000010
expect display.presents is
wait 20
expect library.app.status is 000011
expect display.presents is
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
# Once asks for nothing in a drawing pass: opened, it has its one pass, and in three seconds no other and no
# refresh.
tap library apps
wait 300
tap library app.test.app.once
wait 400
expect library.app.open is 1
expect library.app.status is 000001
expect display.presents not -1
wait 3000
expect library.app.status is 000001
expect display.presents is
# Touched, it asks to be drawn again half a second later. (The touch's two phases are two calls of the module,
# and one drawing pass follows them, at the lift.) The request stands through that pass: 400 ms after the touch
# nothing more has been drawn, and 200 ms later the one pass it asked for has (with no refresh: it changed nothing).
tap 930 1300
wait 300
expect library.app.status is 000002
expect display.presents not -1
wait 100
expect library.app.status is 000002
expect display.presents is
wait 200
expect library.app.status is 000003
expect display.presents is
# The request was taken, and the pass it brought asked for nothing: three seconds more, no pass and no refresh.
wait 3000
expect library.app.status is 000003
expect display.presents is
shot 02-once
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
quit
