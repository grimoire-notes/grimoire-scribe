# settings: status_mode=1
# settings: ink_edges=solid
# Ink edges at Solid (2.3, #320) in a view that draws the paper's lines at the screen's own scale (#308): a notebook
# on Medium rule turned sideways, the whole page fitted at three quarters of its size. Its lines are drawn on whole
# pixels there, two rows of the page's gray each, and solid ink written among them still changes screen pixels only
# to black (editor.rs blit_canvas_ruled with the rule of solid_px), so the frames drawn while writing are the frame
# the page settles to. A stroke between two lines: right after its live frames the panel shows the page as it
# settles (display.stale 0), and the pause after writing sends nothing. A word across two lines: the simulator's fast
# refreshes dither every pixel of their rect, a line's unchanged gray too, where a Kindle's leave an unchanged pixel
# alone (docs/SIM.md, editor.settles_sent), so here its settle sends the rows of the two lines its frames covered,
# and nothing of the ink: between the lines, above and below them, the panel shows the settled page before the
# settle. The lines beside the ink are the ones drawn before it (display.dark over a band the ink does not reach:
# six lines, two rows each).
# rust-only
wait 3600
clock 1700000000
tap library add
tap library add.new-notebook
wait 100
tap library paper.1
wait 100
tap library create
wait 400
expect screen is editor
expect editor.paper is 1
pentap panel tool-picker
pentap 872 561
expect editor.tool is ballpoint
# (a first stroke makes Undo available, which the panel shows with a refresh of its own; between two lines, upright)
pen line 500 380 900 390 20 0.6 0.6
pen leave
wait 2000
expect editor.strokes is 1
expect editor.settles_quiet is 1
expect editor.settles_sent is 0
# Sideways: the whole page, centred, at three quarters of its size, its lines at the screen's scale
rotate 1
wait 1500
expect size is 2480x1860
expect editor.scale is 0.75
expect editor.zoomed is 0
pen leave
wait 2000
expect display.presents not -1
expect display.dark.1500.700.200.400 is 2400
# Between the lines at rows 887 and 950
pen down 900 915 0.5 0 0
pen move 980 925 0.6 0 0
pen move 1060 912 0.6 0 0
wait 30
pen move 1140 926 0.7 0 0
pen move 1220 914 0.7 0 0
pen move 1300 924 0.6 0 0
wait 30
pen move 1380 916 0.6 0 0
expect display.last_fast is ink
expect display.stale.800.700.800.400 is 0
pen up 1400 918
pen leave
expect display.presents has fast
wait 2000
expect editor.strokes is 2
expect display.presents is
expect editor.settles_quiet is 2
expect editor.settles_sent is 0
expect display.stale.800.700.800.400 is 0
# Across them
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
expect display.stale.800.700.800.184 is 0
expect display.stale.800.892.800.55 is 0
expect display.stale.800.955.800.145 is 0
expect display.stale.800.884.800.71 not 0
pen up 1430 890
pen leave
expect display.presents has fast
wait 2000
expect editor.strokes is 3
expect display.presents is ui 893 884 534 71
expect editor.settles_quiet is 2
expect editor.settles_sent is 1
expect display.stale.800.700.800.400 is 0
expect display.dark.1500.700.200.400 is 2400
expect editor.live_matches_render is 1
expect editor.screen_composed is 1
expect editor.layers is ok
shot 01-ruled-sideways
quit
