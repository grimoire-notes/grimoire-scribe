# settings: status_mode=1
# settings: status_size=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# A pass a widget asks for leaves nothing to clean (issue #307), with Gray tick widget of tests/fixtures/kitpanel
# (rust/tests/kitfix: `make kit-fixtures`), which fills a square 200 px a side about 200, 200 of its body, gray 160 on
# its odd passes and 96 on its even ones, and asks in each pass for another a second later. A frame's pass that its
# module asked for is refreshed as the screen's UI (editor.rs widget_runtime_tick), at full depth: the panel holds the
# gray from that refresh on, and no clean follows.
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
tap panel widgets.test.kit.graytickw
wait 400
expect editor.widget.0.rect is 776 106 900 1100
waitfor display.stale.776.106.900.1100 is 0
expect display.presents not -1
expect editor.widget.0.status has p001
waitfor editor.widget.0.status has p002
expect display.presents is ui 776 106 900 1100
expect display.stale.0.0.1860.2480 is 0
# and so on, one refresh of the frame a second, nothing else: each pass in a wait of its own (a wait's time is made
# up in one step at its end on a slow machine, docs/SIM.md: one wait that held two passes gave the second, asked
# for by the first, no time to come due). No pass before its second; the third runs in the 100 ms after it, so
# the fourth is due 900 to 1,000 ms after that wait.
wait 900
expect editor.widget.0.status has p002
expect display.presents is
wait 200
expect editor.widget.0.status has p003
expect display.presents is ui 776 106 900 1100
expect display.stale.0.0.1860.2480 is 0
wait 800
expect editor.widget.0.status has p003
expect display.presents is
wait 300
expect editor.widget.0.status has p004
expect display.presents is ui 776 106 900 1100
expect display.stale.0.0.1860.2480 is 0
quit
