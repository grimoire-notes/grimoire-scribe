# settings: status_mode=1
# settings: ink_edges=solid
# Ink edges at Solid (2.3, #320) in a view that is not 1:1: zoomed in with a pinch, then sideways with the page
# fitted to the screen (landscape_fit, the default). The page is drawn to the screen scaled there, and a solid
# stroke still changes screen pixels only to black (editor.rs blit_solid_edges), so right after its live frames the
# panel shows the page as it settles (display.stale 0 over the word) and the pause after writing sends nothing.
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
pen line 300 400 700 420 20 0.6 0.6
pen leave
wait 2000
expect editor.settles_quiet is 1
# Zoomed in
pinch 600 900 300 750 6
wait 700
expect editor.zoomed is 1
expect editor.zoom not 1.00
pen leave
wait 2000
expect display.presents not -1
pen down 300 900 0.3 0 0
pen move 340 860 0.4 0 0
pen move 380 840 0.5 0 0
pen move 420 860 0.6 0 0
wait 30
pen move 440 920 0.7 0 0
pen move 470 980 0.8 0 0
pen move 520 960 0.8 0 0
wait 30
pen move 560 900 0.7 0 0
pen move 600 860 0.6 0 0
pen move 660 880 0.6 0 0
wait 30
pen move 700 940 0.7 0 0
pen move 760 960 0.9 0 0
pen move 820 900 0.9 0 0
expect display.last_fast is ink
expect display.stale.200.700.1200.500 is 0
pen up 830 890
pen leave
expect display.presents has fast
wait 2000
expect editor.strokes is 2
expect display.presents is
expect editor.settles_quiet is 2
expect editor.settles_sent is 0
expect display.stale.200.300.1200.1200 is 0
expect editor.live_matches_render is 1
shot 01-zoomed
# Sideways: the whole page, centred, at three quarters of its size
rotate 1
wait 600
expect rotation is 1
pen leave
wait 2000
expect display.presents not -1
pen down 900 900 0.3 0 0
pen move 940 860 0.4 0 0
pen move 980 840 0.5 0 0
pen move 1020 860 0.6 0 0
wait 30
pen move 1040 920 0.7 0 0
pen move 1070 980 0.8 0 0
pen move 1120 960 0.8 0 0
wait 30
pen move 1160 900 0.7 0 0
pen move 1200 860 0.6 0 0
pen move 1260 880 0.6 0 0
wait 30
pen move 1300 940 0.7 0 0
pen move 1360 960 0.9 0 0
pen move 1420 900 0.9 0 0
expect display.last_fast is ink
expect display.stale.800.700.800.400 is 0
pen up 1430 890
pen leave
expect display.presents has fast
wait 2000
expect editor.strokes is 3
expect display.presents is
expect editor.settles_quiet is 3
expect editor.settles_sent is 0
expect display.stale.800.700.800.400 is 0
expect editor.live_matches_render is 1
shot 02-sideways
quit
