# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/apphost .
# The app host's rules for a module (issue #247), with the packages of tests/fixtures/apphost (written by
# rust/tests/appfix: `make app-fixtures`). App time is the script's (GRIMOIRE_SIM_CLOCK=script): each `wait`
# moves it on by exactly its milliseconds, so what comes due in a stretch is the same on any machine.
# (the status bar's clock held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
expect library.apps is test.app.asks test.app.counts test.app.later test.app.loop test.app.once test.app.ticker test.app.wide
# Loop's drawing pass never ends. The call is ended when its instructions are used up: the app is closed, a
# toast says that it stopped, and Home is back. No app is left open with nothing drawn.
tap library apps
wait 300
tap library app.test.app.loop
wait 400
expect library.app.open is 0
expect toast is The app stopped: all fuel consumed by WebAssembly
expect screen is library
expect library.sheet is none
shot 01-loop-stopped
# The host and the main loop go on: another app opens, draws, is drawn again for a touch (once: a tap's two
# phases are two calls of the module, and one drawing pass follows them), and leaves by the system swipe; and
# Loop, opened again, stops again.
wait 5200
expect toast is
tap library apps
wait 300
tap library app.test.app.counts
wait 400
expect library.app.open is 1
expect library.app.id is test.app.counts
expect library.app.error is
expect library.app.status is 000001
expect display.dark.40.40.200.100 is 20000
tap 930 1300
wait 300
expect library.app.status is 000002
shot 02-counts
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
tap library apps
wait 300
tap library app.test.app.loop
wait 400
expect library.app.open is 0
expect toast is The app stopped: all fuel consumed by WebAssembly
wait 5200
expect toast is
# Wide shows the status bar and fills black from far above and left of its body to far below and right of it,
# then again through a clip as wide, and again with the clip taken away. Its body, the screen under the bar
# (78 px), is black to its edges, and the bar above it is the launcher's own: the dark pixels of its clock and
# date, as many as before the app was opened.
expect display.dark.0.0.1860.78 is 1604
tap library apps
wait 300
tap library app.test.app.wide
wait 400
expect library.app.open is 1
expect library.app.status-bar is 1
expect library.app.status is wide
expect display.dark.0.78.1860.2402 is 4467720
expect display.dark.0.0.1860.78 is 1604
shot 03-wide
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
# Later's first drawing pass draws its rectangle and asks to be drawn again 100 ms later; its second never ends.
# The first pass is shown and the app is open. When the second is stopped, the host's next round closes the
# app, as it does one whose first pass is stopped: the same toast, and Home is back.
tap library apps
wait 300
tap library app.test.app.later
wait 50
expect library.app.open is 1
expect library.app.id is test.app.later
expect library.app.error is
expect library.app.status is later
expect display.dark.40.40.200.100 is 20000
# (waited for by what it does, not by a time: the pass comes due and is stopped in one round, and the app is
# closed in the next, which a wait that makes its time up in one step at its end does not have)
waitfor library.app.open is 0
expect toast is The app stopped: all fuel consumed by WebAssembly
expect screen is library
expect library.sheet is none
shot 04-later-stopped
quit
