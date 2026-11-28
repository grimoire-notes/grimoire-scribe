# settings: status_mode=1
# settings: ink_edges=smooth
# The Seyes notebook of y_rulings_1_sideways (issue #308) written with the ballpoint, shown sideways with Ink edges
# at Smooth and then at Solid (2.3, #320): the whole page rests at 0.75 with its paper drawn at the screen's scale
# under either setting, so wherever no ink is the screen holds the same lines (display.dark over two bands the ink
# does not reach, the same counts before and after the setting changes, and after more is written at Solid). The
# strokes are y_rulings_1_sideways's, and so are its checks of the screen against its layers.
# rust-only
clock 1700000000
wait 3600
tap library add
wait 200
tap library add.new-notebook
wait 200
tap library paper.20
wait 100
tap library create
wait 400
expect screen is editor
expect editor.paper is 20
expect ink_edges is smooth
pentap panel tool-picker
pentap 872 561
expect editor.tool is ballpoint
# Writing across the lines upright
pen curve 560 700 40 60 0.6 3
pen line 420 980 1500 1040 80 0.3 0.9
pen leave
wait 3000
expect editor.strokes is 2
rotate 1
wait 1500
expect size is 2480x1860
expect editor.scale is 0.75
expect editor.zoomed is 0
expect editor.screen_composed is 1
expect editor.layers is ok
expect display.dark.600.100.1300.300 is 22794
expect display.dark.600.1400.1300.400 is 30375
shot 01-smooth-sideways
rotate 0
wait 1500
expect size is 1860x2480
# Ink edges to Solid, over the notebook (the quick settings' Settings)
pen leave
wait 1100
touch down 930 40
touch move 930 140
wait 150
expect quick is 1
touch up 930 140
wait 100
tap panel quick.settings
wait 300
expect editor.modal is settings
tap library settings.cat.3
wait 100
tap library ink-edges
wait 100
tap library ink-edges.1
wait 300
expect ink_edges is solid
tap library done
wait 300
expect editor.modal is none
expect editor.live_matches_render is 1
rotate 1
wait 1500
expect size is 2480x1860
expect editor.scale is 0.75
expect editor.screen_composed is 1
expect editor.layers is ok
expect display.dark.600.100.1300.300 is 22794
expect display.dark.600.1400.1300.400 is 30375
shot 02-solid-sideways
# Writing sideways, below 100%: the stroke's frames are drawn in parts
pen line 800 1100 1700 1150 60 0.4 0.8
pen curve 1200 1300 30 50 0.6 2
pen leave
wait 3000
expect editor.strokes is 4
expect editor.screen_composed is 1
expect editor.layers is ok
expect editor.live_matches_render is 1
expect display.stale.560.0.1360.1860 is 0
expect display.dark.600.100.1300.300 is 22794
expect display.dark.600.1400.1300.400 is 30375
shot 03-solid-written-sideways
rotate 0
wait 1500
expect size is 1860x2480
expect editor.scale is 1.00
expect editor.screen_composed is 1
quit
