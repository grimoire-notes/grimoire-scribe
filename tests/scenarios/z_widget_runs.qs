# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script
# copy: tests/fixtures/widgetruns .
# Widget modules that run for long (issue #208), with the packages of tests/fixtures/widgetruns (written by
# rust/tests/widgetfix: `make widget-fixtures`). App time is the script's (GRIMOIRE_SIM_CLOCK=script): each
# `wait` moves it on by exactly its milliseconds, so what comes due in a stretch is the same on any machine.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
# Loop's drawing pass never ends. The call is ended when its instructions are used up, and the frame says that
# the widget stopped. Its manifest names no size: the frame is the default one, 220x160.
tap panel widgets
wait 300
expect editor.widgets.open is 1
tap panel widgets.test.runs.loop
wait 400
expect editor.widgets is 1 test.runs.loop
expect editor.widget.0.rect is 1456 28 220 160
expect editor.widget.0.error is The widget stopped: all fuel consumed by WebAssembly
expect editor.widget.0.hits is 0
# A frame whose module failed says the reason first, then whose widget it is. This body, 192x74 at 1470,100, has
# room for one line: the start of the reason ("The widget s..."), at the top of the body, and nothing under it or
# under the frame.
expect editor.widget.0.says is The widget stopped: all fuel consumed by WebAssembly | Loop v1.0.0 · Grimoire tests
expect display.dark.1470.100.192.40 is 1576
expect display.dark.1470.140.192.34 is 0
expect display.dark.1456.192.220.40 is 0
shot 01-loop
# (the frame is still a frame: it closes)
tap panel widget.0.close
wait 300
expect editor.widgets is 0
# Ticker counts its drawing passes in its status line, draws a rectangle and asks to be drawn again a second
# later. A frame whose pass is due is drawn again and its rectangle refreshed once; while nothing is due, nothing
# is drawn and nothing is refreshed. (Each `waitfor` ends in the round the pass it names ran in, so the next
# pass is due one second of app time after it.)
tap panel widgets
wait 300
tap panel widgets.test.runs.ticker
wait 400
expect editor.widgets is 1 test.runs.ticker
expect editor.widget.0.rect is 1456 28 220 160
expect editor.widget.0.error is
expect editor.widget.0.status is 1
waitfor editor.widget.0.status is 2
expect display.presents not -1
# 900 ms after a pass: no pass, no refresh.
wait 900
expect editor.widget.0.status is 2
expect display.presents is
# 200 ms more: the pass that was due, and one refresh of the frame's rectangle.
wait 200
expect editor.widget.0.status is 3
expect display.presents is ui 1456 28 220 160
# And so on, a pass and a refresh a second.
waitfor editor.widget.0.status is 4
expect display.presents is ui 1456 28 220 160
wait 900
expect editor.widget.0.status is 4
expect display.presents is
wait 200
expect editor.widget.0.status is 5
expect display.presents is ui 1456 28 220 160
shot 02-ticker
# Minimized, the frame draws no body: its module has no pass and its square is not refreshed, however long the
# request has been due. Three seconds: nothing.
tap panel widget.0.min
wait 300
expect editor.widget.0.state is minimized
expect editor.widget.0.rect is 1456 28 112 112
expect display.presents not -1
wait 3000
expect editor.widget.0.status is 5
expect display.presents is
# Restored, the frame is drawn (the pass its request waited for), and then again a second later, as before.
tap panel widget.0.title
wait 300
expect editor.widget.0.state is normal
expect editor.widget.0.status is 6
expect display.presents not -1
wait 600
expect editor.widget.0.status is 6
expect display.presents is
wait 200
expect editor.widget.0.status is 7
expect display.presents is ui 1456 28 220 160
tap panel widget.0.close
wait 300
expect editor.widgets is 0
# Bare is the same module in a package whose manifest declares no capability (no install writes such a package
# now; an earlier one could have). Asking for a redraw needs the capability to draw: its module has the one pass
# that opening the frame gives it, and in three seconds no other, and nothing is refreshed.
tap panel widgets
wait 300
tap panel widgets.test.runs.bare
wait 400
expect editor.widgets is 1 test.runs.bare
expect editor.widget.0.error is
expect editor.widget.0.status is 1
expect display.dark.1470.100.192.74 is 0
expect display.presents not -1
wait 3000
expect editor.widget.0.status is 1
expect display.presents is
tap panel widget.0.close
wait 300
expect editor.widgets is 0
quit
