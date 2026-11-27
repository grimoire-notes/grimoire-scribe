# settings: status_mode=1
# settings: ink_edges=solid
# Ink edges at Solid (2.3, #320): a ballpoint word on blank paper. Its live frames put only black on the panel, so
# right after them the panel already shows the page as it will settle (display.stale 0 over the word), and the pause
# after writing sends nothing (no refresh at all: editor.settles_quiet counts it). The page is the word drawn afresh.
# (A first stroke before it makes Undo available, which the panel shows with a refresh of its own.)
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
expect editor.strokes is 1
expect editor.settles_quiet is 1
expect editor.settles_sent is 0
expect display.stale.200.300.1200.1200 is 0
# The word: its samples in batches, each a live frame
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
shot 01-live-solid
pen up 830 890
pen leave
expect display.stale.200.700.1200.500 is 0
expect display.presents has fast
wait 2000
expect editor.strokes is 2
expect display.presents is
expect editor.settles_quiet is 2
expect editor.settles_sent is 0
expect display.stale.200.300.1200.1200 is 0
expect editor.live_matches_render is 1
shot 02-settled-solid
quit
