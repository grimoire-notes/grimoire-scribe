# settings: status_mode=1
# settings: status_size=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# A frame's tap is drawn whole when another frame's redraw draws part of it first (issue #272), with the packages of
# tests/fixtures/kitpanel (rust/tests/kitfix: `make kit-fixtures`). Black widget (frame 0) fills a black square 100 px
# a side about its last touch; Ask widget (frame 1) asks for a pass a second after each, and is moved over frame 0's
# lower part. A finger held on frame 0 across frame 1's due time: frame 1's pass waits for the lift, and the lift's
# tick draws it first, which composes the part of frame 0's body under it, frame 0's module drawing there too. That
# partial pass does not answer the pass the TAP owed (editor.rs draw_widget_frame): frame 0's own comes in the same
# tick, whole, and the square is on the screen and the panel. z_kit_input_overlap_before.qs taps before the due time.
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
tap panel widgets.test.kit.blackw
wait 400
tap panel widgets
wait 300
tap panel widgets.test.kit.askw
wait 400
drag touch 1100 130 1100 830 7
wait 1500
expect editor.widget.0.rect is 776 106 900 1100
expect editor.widget.1.rect is 754 828 900 1100
waitfor display.stale.0.100.1860.2380 is 0
waitfor editor.widget.1.status has p006
wait 700
# a tap on frame 0's body above frame 1, held across frame 1's due time (1,000 ms after its pass)
expect display.dark.1250.350.100.100 is 0
touch down 1300 400
wait 400
touch up 1300 400
wait 300
expect editor.widget.0.status has t01
expect display.dark.1250.350.100.100 is 10000
expect display.stale.776.106.900.1100 is 0
expect display.stale.0.100.1860.2380 is 0
quit
