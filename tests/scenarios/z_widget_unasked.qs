# settings: status_mode=1
# settings: shortcuts_enabled=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script
# copy: tests/fixtures/widgetruns .
# A widget frame's timed redraw and the platform (issue #280), with Ticker of tests/fixtures/widgetruns (written by
# rust/tests/widgetfix: `make widget-fixtures`), which asks to be drawn again a second after each pass. Its refresh
# is one nobody asked for, and app_flush says so to the platform (display_unasked, the runner's display.unasked),
# which may leave out a cleaning flash of its own for it. A page turned is a refresh something asked for, and goes
# without. The redraw is a ui of the frame's rectangle as before (z_widget_runs.qs holds its passes and refreshes).
# App time is the script's (GRIMOIRE_SIM_CLOCK=script): each `wait` moves it on by exactly its milliseconds.
# (the status bar's clock held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap library add
tap library add.new-notebook
wait 100
tap 1564 2254
wait 300
expect screen is editor
tap panel widgets
wait 300
tap panel widgets.test.runs.ticker
wait 400
expect editor.widgets is 1 test.runs.ticker
expect editor.widget.0.rect is 1456 106 220 160
expect editor.widget.0.status is 1
# (the `waitfor` ends in the round the second pass ran in, so the third is due one second of app time after it)
waitfor editor.widget.0.status is 2
expect display.presents not -1
expect display.unasked not -1
# A key's next page, 200 ms after the pass (on the last page it adds one): asked for, so no hint. The new page's
# drawing draws the frame too, a pass of the module's in the refresh the page turn asked for.
wait 200
key 106 0
wait 300
expect editor.page is 2
expect editor.widget.0.status is 3
expect display.unasked is 0
expect display.presents not -1
# The next pass, when its time comes: one refresh of the frame's rectangle, and the hint with it.
waitfor editor.widget.0.status is 4
expect display.presents is ui 1456 106 220 160
expect display.unasked is 1
# And with the next pass, again.
waitfor editor.widget.0.status is 5
expect display.presents is ui 1456 106 220 160
expect display.unasked is 1
shot 01-ticker
quit
