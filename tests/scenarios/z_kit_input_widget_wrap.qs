# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC GRIMOIRE_SIM_UPTIME_MS=4294961000
# copy: tests/fixtures/kitinput .
# z_kit_input_widget.qs with app time started 6,296 ms before 2^32 ms (49.7 days awake): the clock wraps to 0 during
# the drag of 20 moves, with the same passes and refreshes (issue #272). From the next line on this file is
# z_kit_input_widget.qs from its fifth line on (rust/tests/test_kit_input.rs checks it).
# The package kit's input in a widget's frame (issue #272), with the packages of tests/fixtures/kitinput (written by
# rust/tests/kitfix: `make kit-fixtures`). Each widget counts in its status line what z_kit_input_app.qs's apps do,
# with widget_tap's calls for n, and fills a black square about the place of its last touch (200, 200 at first),
# which widget_tap does not move. Input widget has widget_tap and kit_event, which answers every event, so its taps
# are TAPs; Tap widget widget_tap alone; Quiet widget kit_event alone, which answers a TAP alone. The frame of each is
# 900 x 1100 at 776, 106, its body under its title bar. A drawing pass input owes is drawn once for all the input
# since the last, none while the contact may still be a tap, at most one every 120 ms while it is down, and at once
# after the lift; and only the part of the body it changed is refreshed. What the fast refreshes covered gets one
# clean refresh after the editor's pause (editor.rs du_wear_owe), whose compose draws the frame's module once more.
# (On main a tap was a pass and a refresh of the whole frame, a hold's lift a tap, and a drag nothing but a pass and a
# refresh of the whole frame at its lift.)
# App time is the script's (GRIMOIRE_SIM_CLOCK=script); the status bar's clock is held still, in UTC.
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
tap panel widgets.test.kit.inputw
wait 400
expect editor.widgets is 1 test.kit.inputw
expect editor.widget.0.rect is 776 106 900 1100
expect editor.widget.0.status is p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect editor.widget.0.hits is 2
expect display.presents not -1
# A tap on hit region 1: a TAP with its id (no widget_tap), one pass, and a refresh of the square's two places.
tap 1000 500
wait 300
expect editor.widget.0.status is p002 n000 t01 r00 s00 m00 e00 c00 q000 k00 a00 g00 h2
expect display.presents is ui 940 328 110 222
# A long press: at 600 ms a PRESS and a pass, refreshed fast; its lift is no tap and owes nothing, and the clean of
# what the fast refresh covered draws the frame's body there once more.
touch down 1300 800
wait 500
expect editor.widget.0.status is p002 n000 t01 r00 s00 m00 e00 c00 q000 k00 a00 g00 h2
expect display.presents is
wait 200
expect editor.widget.0.status is p003 n000 t01 r01 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is fast 950 450 400 400
touch up 1300 800
wait 300
expect editor.widget.0.status is p004 n000 t01 r01 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is clean 950 450 400 400
# A drag of 20 moves 20 ms apart in the body: DRAG_START once past the slop, a pass on each of the pace's turns
# (fast refreshes of the part changed), and at the lift DRAG_END and a UI refresh: 5 passes; then the clean (a pass
# more).

touch down 850 900
wait 20
touch move 887 900
wait 20
touch move 925 900
wait 20
touch move 962 900
wait 20
touch move 1000 900
wait 20
touch move 1037 900
wait 20
touch move 1075 900
wait 20
touch move 1112 900
wait 20
touch move 1150 900
wait 20
touch move 1187 900
wait 20
touch move 1225 900
wait 20
touch move 1262 900
wait 20
touch move 1300 900
wait 20
touch move 1337 900
wait 20
touch move 1375 900
wait 20
touch move 1412 900
wait 20
touch move 1450 900
wait 20
touch move 1487 900
wait 20
touch move 1525 900
wait 20
touch move 1562 900
wait 20
touch move 1600 900
wait 20
touch up 1600 900
wait 500
expect editor.widget.0.status is p010 n000 t01 r01 s01 m05 e01 c00 q019 k00 a00 g00 h0
expect display.presents is fast 875 750 475 200, fast 875 850 287 100, fast 1062 850 325 100, fast 1287 850 325 100, ui 1512 850 138 100, clean 875 750 737 200
# A flood: 2,000 moves in one batch in the body: the samples go to kit_event on the pace's turns alone (the main
# loop reads at most 64 events between two of its ticks), one DRAG_MOVE on the first turn and at the lift one with
# the newest 256 (289 points), and two passes for the 2,002 phases; then the clean (a pass more).
drag touch 850 1100 1600 1100 2000
wait 500
expect editor.widget.0.status is p013 n000 t01 r01 s02 m07 e02 c00 q308 k00 a00 g00 h0
expect display.presents is fast 872 850 778 300, ui 872 1050 778 100, clean 872 850 778 300
shot 01-input-widget
tap panel widget.0.close
wait 300
expect editor.widgets is 0
expect display.presents not -1
# Tap widget has no kit_event: a lift within the slop calls widget_tap, held or not, as before; its pass draws the
# same picture, so nothing is refreshed (main refreshed the whole frame); a drag calls nothing and draws nothing.
tap panel widgets
wait 300
tap panel widgets.test.kit.tapw
wait 400
expect display.presents not -1
tap 1000 500
wait 300
expect editor.widget.0.status is p002 n001 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h2
expect display.presents is
touch down 1300 800
wait 700
touch up 1300 800
wait 300
expect editor.widget.0.status is p003 n002 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is

touch down 850 900
wait 20
touch move 887 900
wait 20
touch move 925 900
wait 20
touch move 962 900
wait 20
touch move 1000 900
wait 20
touch move 1037 900
wait 20
touch move 1075 900
wait 20
touch move 1112 900
wait 20
touch move 1150 900
wait 20
touch move 1187 900
wait 20
touch move 1225 900
wait 20
touch move 1262 900
wait 20
touch move 1300 900
wait 20
touch move 1337 900
wait 20
touch move 1375 900
wait 20
touch move 1412 900
wait 20
touch move 1450 900
wait 20
touch move 1487 900
wait 20
touch move 1525 900
wait 20
touch move 1562 900
wait 20
touch move 1600 900
wait 20
touch up 1600 900
wait 500
expect editor.widget.0.status is p003 n002 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is
tap panel widget.0.close
wait 300
expect editor.widgets is 0
expect display.presents not -1
# Quiet widget has kit_event alone, which answers a TAP alone: a press and a drag call it and owe nothing.
tap panel widgets
wait 300
tap panel widgets.test.kit.quietw
wait 400
expect display.presents not -1
tap 1000 500
wait 300
expect editor.widget.0.status is p002 n000 t01 r00 s00 m00 e00 c00 q000 k00 a00 g00 h2
expect display.presents is ui 940 328 110 222
touch down 1300 800
wait 700
touch up 1300 800
wait 300
expect display.presents is

touch down 850 900
wait 20
touch move 887 900
wait 20
touch move 925 900
wait 20
touch move 962 900
wait 20
touch move 1000 900
wait 20
touch move 1037 900
wait 20
touch move 1075 900
wait 20
touch move 1112 900
wait 20
touch move 1150 900
wait 20
touch move 1187 900
wait 20
touch move 1225 900
wait 20
touch move 1262 900
wait 20
touch move 1300 900
wait 20
touch move 1337 900
wait 20
touch move 1375 900
wait 20
touch move 1412 900
wait 20
touch move 1450 900
wait 20
touch move 1487 900
wait 20
touch move 1525 900
wait 20
touch move 1562 900
wait 20
touch move 1600 900
wait 20
touch up 1600 900
wait 500
expect editor.widget.0.status is p002 n000 t01 r00 s00 m00 e00 c00 q000 k00 a00 g00 h2
expect display.presents is
tap 1400 500
wait 300
expect editor.widget.0.status is p003 n000 t02 r01 s01 m05 e01 c00 q019 k00 a00 g00 h3
expect display.presents is ui 950 450 500 100
tap panel widget.0.close
wait 300
expect editor.widgets is 0
quit
