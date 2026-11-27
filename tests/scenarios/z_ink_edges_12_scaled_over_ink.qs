# settings: status_mode=1
# settings: ink_edges=solid
# Ink edges at Solid (2.3, #320), a highlighted ballpoint word in a view that is not 1:1. The highlighter goes over
# the word three times, 2 px apart, which leaves a band darker than mid-gray everywhere (display.dark over its middle
# is the rect's whole area). Sideways (the page fitted, three quarters of its size) and pinched in, the band stays so:
# no pixel beside the word's black is lighter than the highlight around it (editor.rs blit_solid_edges leaves
# blit_canvas's blend beside other ink), also after a second and a third word are written across the highlight in
# those views and have settled. The page is the strokes drawn afresh at each step.
# rust-only
wait 3600
clock 1700000000
tap library add
tap library add.new-notebook
wait 100
tap library paper.4
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 4
pentap panel tool-picker
pentap 872 561
expect editor.tool is ballpoint
pen line 400 870 460 930 12 0.6 0.6
pen line 460 930 520 870 12 0.6 0.6
pen line 520 870 580 930 12 0.6 0.6
pen line 580 930 640 870 12 0.6 0.6
pen line 640 870 700 930 12 0.6 0.6
pen line 700 930 760 870 12 0.6 0.6
pen line 760 870 820 930 12 0.6 0.6
pen line 820 930 880 870 12 0.6 0.6
pen line 880 870 940 930 12 0.6 0.6
pen line 940 930 1000 870 12 0.6 0.6
pen line 400 900 1000 902 30 0.6 0.6
pen leave
wait 2000
expect editor.strokes is 11
expect editor.settles_quiet is 1
expect editor.settles_sent is 0
# The highlighter over the word
pentap panel tool-picker
pentap 1504 727
expect editor.tool is highlighter
pen line 300 900 1100 900 40 0.8 0.8
pen line 302 900 1102 900 40 0.8 0.8
pen line 304 900 1104 900 40 0.8 0.8
pen leave
wait 2000
expect editor.strokes is 14
expect display.stale.200.800.1100.200 is 0
expect display.dark.450.880.500.40 is 20000
expect editor.live_matches_render is 1
shot 01-highlighted
pentap panel tool-picker
pentap 872 561
expect editor.tool is ballpoint
# Sideways: the whole page, centred, at three quarters of its size
rotate 1
wait 600
expect rotation is 1
pen leave
wait 2000
expect display.stale.700.500.800.400 is 0
expect display.dark.900.660.300.30 is 9000
shot 02-sideways
# A second word, written across the highlight there
pen line 800 662 1300 688 40 0.6 0.6
pen leave
wait 2000
expect editor.strokes is 15
expect display.stale.700.500.800.400 is 0
expect display.dark.900.660.300.30 is 9000
expect editor.live_matches_render is 1
shot 03-sideways-written
# Upright again and pinched in
rotate 0
wait 600
expect rotation is 0
pinch 600 900 300 750 6
wait 700
expect editor.zoomed is 1
expect editor.zoom is 2.00
pen leave
wait 2000
expect display.stale.100.700.1600.400 is 0
expect display.dark.400.860.900.80 is 72000
shot 04-pinched
# A third word, written across the highlight there
pen line 300 870 1500 930 60 0.6 0.6
pen leave
wait 2000
expect editor.strokes is 16
expect display.stale.100.700.1600.400 is 0
expect display.dark.400.860.900.80 is 72000
expect editor.live_matches_render is 1
shot 05-pinched-written
quit
