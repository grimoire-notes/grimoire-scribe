# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC GRIMOIRE_SIM_UPTIME_MS=2147480000
# copy: tests/fixtures/kitinput .
# z_kit_input_app.qs with app time started 3,648 ms before 2^31 ms (24.8 days awake): it passes 2^31 in the launch's
# wait, so every press and drag below comes past it, with the same passes and refreshes (issue #272). From the next
# line on this file is z_kit_input_app.qs from its fifth line on (rust/tests/test_kit_input.rs checks it).
# The package kit's input in an app (issue #272), with the packages of tests/fixtures/kitinput (written by
# rust/tests/kitfix: `make kit-fixtures`). Each app counts in its status line its drawing passes (p), the calls of
# app_pointer (n), the kit's events by kind (t TAP, r PRESS, s DRAG_START, m DRAG_MOVE, e DRAG_END, k KEY, g SETTINGS),
# the cancelled drags (c), the points its DRAG_MOVEs gathered (q), app_key's calls (a) and the hit region of its last
# touch plus one (h); it fills a black square about the place of its last touch. Input app has app_pointer and
# kit_event, which answers every event; Pointer app app_pointer alone; Quiet app kit_event alone, which answers a TAP
# alone. A touch still calls the module at once; the drawing pass it owes is drawn once for all the input since the
# last, none while the contact may still be a tap, at most one every 120 ms while it is down (a fast refresh), and at
# once after the lift (a UI refresh); and only the part of the screen the pass changed is refreshed. What the fast
# refreshes covered gets one clean refresh 150 ms after the lift (apphost.rs apphost_wear_tick). (On main every phase
# was a pass and a refresh of the whole screen: a tap two of each, a drag of 20 moves 22.)
# App time is the script's (GRIMOIRE_SIM_CLOCK=script); the status bar's clock is held still, in UTC.
clock 1700000000
wait 3600
expect library.apps is test.kit.input test.kit.pointer test.kit.quiet
tap library apps
wait 300
tap library app.test.kit.input
wait 400
expect library.app.open is 1
expect library.app.status is p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect library.app.hits is 2
expect display.presents not -1
# A tap: its two phases call app_pointer, then a TAP; one pass, and a refresh of the square's old and new places.
tap 600 1000
wait 300
expect library.app.status is p002 n002 t01 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is ui 150 228 500 822
# A tap on hit region 1, by its name: the TAP carries its id.
tap library app.hit.1
wait 300
expect library.app.status is p003 n004 t02 r00 s00 m00 e00 c00 q000 k00 a00 g00 h2
expect display.presents is ui 150 228 500 822
# A long press: no pass while the contact may still be a tap (the status line is the last pass's); at 600 ms a
# PRESS and a pass (fast, the contact is down); its lift is no tap, and its pass changes nothing on the screen: no
# refresh for it, and then the clean refresh of what the press's fast refresh covered.
touch down 900 1400
wait 500
expect library.app.status is p003 n004 t02 r00 s00 m00 e00 c00 q000 k00 a00 g00 h2
expect display.presents is
wait 200
expect library.app.status is p004 n005 t02 r01 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is fast 150 228 800 1222
touch up 900 1400
wait 300
expect library.app.status is p005 n006 t02 r01 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is clean 150 228 800 1222
# A drag of 20 moves 20 ms apart: DRAG_START, a pass on each of the pace's turns with the samples gathered since
# (a fast refresh of the part changed), and at the lift the last DRAG_MOVE, DRAG_END and a UI refresh: 5 passes
# for the 22 phases; then the clean refresh of all the fast refreshes covered.

touch down 300 600
wait 20
touch move 360 600
wait 20
touch move 420 600
wait 20
touch move 480 600
wait 20
touch move 540 600
wait 20
touch move 600 600
wait 20
touch move 660 600
wait 20
touch move 720 600
wait 20
touch move 780 600
wait 20
touch move 840 600
wait 20
touch move 900 600
wait 20
touch move 960 600
wait 20
touch move 1020 600
wait 20
touch move 1080 600
wait 20
touch move 1140 600
wait 20
touch move 1200 600
wait 20
touch move 1260 600
wait 20
touch move 1320 600
wait 20
touch move 1380 600
wait 20
touch move 1440 600
wait 20
touch move 1500 600
wait 20
touch up 1500 600
wait 500
expect library.app.status is p010 n028 t02 r01 s01 m05 e01 c00 q020 k00 a00 g00 h0
expect display.presents is fast 310 550 640 900, fast 310 550 400 100, fast 610 550 460 100, fast 970 550 460 100, ui 1330 550 220 100, clean 310 550 1120 900
# A flood: 2,000 moves in one batch. Every phase still calls app_pointer (n counts to 1,000 and again), but the
# samples go to kit_event on the pace's turns alone (the main loop reads at most 64 events between two of its
# ticks, and the script's clock does not move within a batch): one DRAG_MOVE on the first turn after the start, and
# at the lift one with the newest 256 (284 points in all); and two passes for the 2,002 phases, and the clean.
drag touch 300 1600 1500 1600 2000
wait 500
expect library.app.status is p012 n030 t02 r01 s02 m07 e02 c00 q304 k00 a00 g00 h0
expect display.presents is fast 326 550 1224 1100, ui 326 1550 1224 100, clean 326 550 1224 1100
shot 01-input-app
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
# Pointer app has no kit_event: the same passes and refreshes, which its app_pointer calls owe, and the same cleans.
tap library apps
wait 300
tap library app.test.kit.pointer
wait 400
expect library.app.open is 1
expect display.presents not -1
tap 600 1000
wait 300
expect library.app.status is p002 n002 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is ui 150 228 500 822
touch down 900 1400
wait 700
expect library.app.status is p003 n003 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is fast 550 950 400 500
touch up 900 1400
wait 300
expect library.app.status is p004 n004 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is clean 550 950 400 500

touch down 300 600
wait 20
touch move 360 600
wait 20
touch move 420 600
wait 20
touch move 480 600
wait 20
touch move 540 600
wait 20
touch move 600 600
wait 20
touch move 660 600
wait 20
touch move 720 600
wait 20
touch move 780 600
wait 20
touch move 840 600
wait 20
touch move 900 600
wait 20
touch move 960 600
wait 20
touch move 1020 600
wait 20
touch move 1080 600
wait 20
touch move 1140 600
wait 20
touch move 1200 600
wait 20
touch move 1260 600
wait 20
touch move 1320 600
wait 20
touch move 1380 600
wait 20
touch move 1440 600
wait 20
touch move 1500 600
wait 20
touch up 1500 600
wait 500
expect library.app.status is p009 n026 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is fast 310 550 640 900, fast 310 550 400 100, fast 610 550 460 100, fast 970 550 460 100, ui 1330 550 220 100, clean 310 550 1120 900
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
# Quiet app has kit_event alone, which answers a TAP alone: a press and a drag call it but owe no pass, so nothing
# is drawn and nothing refreshed for them; the next tap's pass shows them counted.
tap library apps
wait 300
tap library app.test.kit.quiet
wait 400
expect library.app.open is 1
expect display.presents not -1
tap 600 1000
wait 300
expect library.app.status is p002 n000 t01 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is ui 150 228 500 822
touch down 900 1400
wait 700
touch up 900 1400
wait 300
expect library.app.status is p002 n000 t01 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is

touch down 300 600
wait 20
touch move 360 600
wait 20
touch move 420 600
wait 20
touch move 480 600
wait 20
touch move 540 600
wait 20
touch move 600 600
wait 20
touch move 660 600
wait 20
touch move 720 600
wait 20
touch move 780 600
wait 20
touch move 840 600
wait 20
touch move 900 600
wait 20
touch move 960 600
wait 20
touch move 1020 600
wait 20
touch move 1080 600
wait 20
touch move 1140 600
wait 20
touch move 1200 600
wait 20
touch move 1260 600
wait 20
touch move 1320 600
wait 20
touch move 1380 600
wait 20
touch move 1440 600
wait 20
touch move 1500 600
wait 20
touch up 1500 600
wait 500
expect library.app.status is p002 n000 t01 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents is
tap 1200 1000
wait 300
expect library.app.status is p003 n000 t02 r01 s01 m05 e01 c00 q020 k00 a00 g00 h0
expect display.presents is ui 550 950 700 100
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
quit
