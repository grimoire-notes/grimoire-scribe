# settings: status_mode=1
# settings: status_size=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# What a frame's passes refreshed fast while a finger was down is cleaned after the lift (issue #272), with Gray widget
# of tests/fixtures/kitpanel (rust/tests/kitfix: `make kit-fixtures`), which fills a gray (128) square 200 px a side
# about its last touch. Once the contact has lifted, all the fast refreshes covered is owed one Clean refresh, whatever
# its size, which the editor's wear cleaning sends after its pause (editor.rs du_wear_owe, du_wear_tick), and the panel
# holds the gray. A press whose lift changes nothing; a drag that pauses before its lift; then the panel 30 s later.
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
tap panel widgets.test.kit.grayw
wait 400
expect editor.widget.0.rect is 776 106 900 1100
waitfor display.stale.776.106.900.1100 is 0
expect display.presents not -1
touch down 1300 800
wait 700
expect display.presents is fast 890 278 510 622
expect display.stale.1150.650.300.300 not 0
touch up 1300 800
wait 3000
expect editor.widget.0.status has r01
expect display.presents is clean 890 278 510 622
expect display.stale.776.106.900.1100 is 0
touch down 900 1000
wait 20
touch move 1000 1000
wait 20
touch move 1100 1000
wait 20
touch move 1200 1000
wait 20
touch move 1300 1000
wait 20
touch move 1400 1000
wait 500
touch up 1400 1000
wait 3000
expect editor.widget.0.status has e01
expect display.presents is fast 900 700 500 400, fast 900 900 600 200, clean 900 700 600 400
expect display.stale.776.106.900.1100 is 0
wait 30000
expect display.presents is
expect display.stale.776.106.900.1100 is 0
quit
