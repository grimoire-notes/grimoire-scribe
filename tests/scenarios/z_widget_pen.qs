# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script
# copy: tests/fixtures/widgetruns .
# A widget frame's due redraw and the pen (issue #243), with Ticker of tests/fixtures/widgetruns (written by
# rust/tests/widgetfix: `make widget-fixtures`), which counts its drawing passes in its status line and asks to be
# drawn again a second after each. A pass that is due waits while the pen is near, while it writes, and after a
# lift with the pen still near, by the clock corner's rule (status.rs, status_busy): no pass and no refresh of
# the frame, however long. When the pen has left, the one pass that waited and one refresh of the frame's
# rectangle, and the next pass a second later, as with no pen (z_widget_runs.qs holds that part with a finger).
# App time is the script's (GRIMOIRE_SIM_CLOCK=script): each `wait` moves it on by exactly its milliseconds.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
tap panel widgets
wait 300
expect editor.widgets.open is 1
tap panel widgets.test.runs.ticker
wait 400
expect editor.widgets is 1 test.runs.ticker
expect editor.widget.0.rect is 1456 28 220 160
expect editor.widget.0.error is
expect editor.widget.0.status is 1
# (the `waitfor` ends in the round the second pass ran in, so the third is due one second of app time after it)
waitfor editor.widget.0.status is 2
expect display.presents not -1
# Near, over the page, for three seconds: no pass, no refresh.
pen hover 700 1400
wait 3000
expect editor.widget.0.status is 2
expect display.presents is
# Gone: the pass that waited, once, and its refresh; then a pass and a refresh a second, as before.
pen leave
wait 100
expect editor.widget.0.status is 3
expect display.presents is ui 1456 28 220 160
wait 800
expect editor.widget.0.status is 3
expect display.presents is
wait 200
expect editor.widget.0.status is 4
expect display.presents is ui 1456 28 220 160
# A stroke on the page, held down past the second: no pass, and the only refresh is the stroke's own fast one.
pen down 700 1400 0.6
pen move 760 1440 0.6
wait 1200
expect editor.owner is draw pen
expect display.pen_mode is 1
expect editor.widget.0.status is 4
expect display.presents is fast 695 1395 72 52
# Lifted, the pen still near: no pass and no refresh.
pen up 800 1450
wait 300
expect display.pen_mode is 0
expect editor.widget.0.status is 4
expect display.presents is
# The page settles under the pen 1.5 s after the lift (the editor's own refresh of the stroke and of the toolbar,
# whose Undo the stroke lit: not counted here), and the frame still waits, for as long as the pen is near.
wait 1500
expect editor.widget.0.status is 4
expect display.presents not -1
wait 1000
expect editor.widget.0.status is 4
expect display.presents is
# Gone: one pass and one refresh of the frame, and the next a second later.
pen leave
wait 100
expect editor.widget.0.status is 5
expect display.presents is ui 1456 28 220 160
wait 800
expect editor.widget.0.status is 5
expect display.presents is
wait 200
expect editor.widget.0.status is 6
expect display.presents is ui 1456 28 220 160
shot 01-after-the-pen
# A pen's tap on the frame itself is drawn at its lift, as a finger's is (the frame's own drawing, not a pass that
# came due): one pass and one refresh at once, with the pen still near. The pass that then comes due waits for
# the pen like any other.
pentap panel widget.0.body
wait 300
expect editor.widget.0.status is 7
expect display.presents is ui 1456 28 220 160
wait 1200
expect editor.widget.0.status is 7
expect display.presents is
pen leave
wait 100
expect editor.widget.0.status is 8
expect display.presents is ui 1456 28 220 160
tap panel widget.0.close
wait 300
expect editor.widgets is 0
quit
