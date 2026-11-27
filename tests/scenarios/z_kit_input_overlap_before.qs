# settings: status_mode=1
# settings: status_size=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# z_kit_input_overlap.qs's frames and tap, the tap held and lifted before frame 1's pass is due (issue #272): frame
# 0's pass is drawn whole at the lift, and the square is on the screen and the panel.
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
wait 100
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
