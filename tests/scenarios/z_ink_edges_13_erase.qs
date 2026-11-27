# settings: status_mode=1
# settings: ink_edges=solid
# settings: eraser_mode=1
# Ink edges at Solid (2.3, #320), erasing in the app. Four ballpoint lines on blank paper: the precise eraser goes
# through the first and along the edge of the second, the stroke eraser takes the third away, and white ink of the
# ballpoint is traced along the fourth. A solid stroke's black is restored to the paper whole or not at all, so the
# live frames of each pass put only black and white on the panel: right after them the panel already shows the page
# as it will settle (display.stale 0, read while the pen is still down), no gray rim is left for the pause to send
# (editor.settles_quiet counts it, editor.settles_sent stays), and the page is the strokes drawn afresh. Then undo
# and redo of the white ink, and the notebook closed (compacted) and opened again: the same pixels (display.dark over
# each line, editor.matches_closed), the page drawn afresh each time.
# (Each pause ends with "wait 30": the settle is taken in the loop's pass after the one that put the stroke's last
# frames on the panel, so a loop that overran the whole pause still settles before the checks are read.)
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
expect settings.eraser_mode is 1
pentap panel tool-picker
pentap 872 561
expect editor.tool is ballpoint
# (a first stroke makes Undo available, which the panel shows with a refresh of its own)
pen line 300 400 700 420 20 0.6 0.6
pen leave
wait 2000
wait 30
expect editor.settles_quiet is 1
# The four lines
pen line 300 900 1300 910 40 0.6 0.6
pen line 300 1200 1300 1200 40 0.6 0.6
pen line 300 1500 1300 1510 40 0.6 0.6
pen line 300 1800 1300 1800 40 0.6 0.6
pen leave
wait 2000
wait 30
expect editor.strokes is 5
expect editor.settles_quiet is 2
expect editor.settles_sent is 0
expect display.stale.200.300.1200.1700 is 0
expect display.dark.250.860.1100.100 is 5219
expect display.dark.250.1160.1100.100 is 6014
expect display.dark.250.1460.1100.100 is 5219
expect display.dark.250.1760.1100.100 is 6014
shot 01-four-lines
# The precise eraser (the pen's eraser end) through the first line, twice
eraser down 600 820 0.6 0 0
eraser move 610 860 0.6 0 0
eraser move 620 900 0.6 0 0
wait 30
eraser move 630 940 0.6 0 0
eraser move 640 980 0.6 0 0
wait 30
eraser move 900 980 0.6 0 0
eraser move 910 940 0.6 0 0
wait 30
eraser move 920 900 0.6 0 0
eraser move 930 860 0.6 0 0
eraser move 940 820 0.6 0 0
expect display.stale.200.300.1200.1700 is 0
eraser up 940 820
pen leave
wait 2000
wait 30
expect editor.strokes is 6
expect editor.erase_strokes is 1
expect editor.settles_quiet is 3
expect editor.settles_sent is 0
expect display.stale.200.300.1200.1700 is 0
expect display.dark.250.860.1100.100 is 4224
expect display.dark.590.860.60.100 is 0
expect editor.live_matches_render is 1
shot 02-erased-through
# Along the second line's upper edge: it loses a part of its width there, whole, and keeps the rest black
eraser down 350 1170 0.4 0 0
eraser move 450 1170 0.4 0 0
eraser move 550 1170 0.4 0 0
wait 30
eraser move 650 1170 0.4 0 0
eraser move 750 1170 0.4 0 0
eraser move 850 1170 0.4 0 0
wait 30
eraser move 950 1170 0.4 0 0
eraser move 1050 1170 0.4 0 0
eraser move 1150 1170 0.4 0 0
expect display.stale.200.300.1200.1700 is 0
eraser up 1150 1170
pen leave
wait 2000
wait 30
expect editor.strokes is 7
expect editor.erase_strokes is 2
expect editor.settles_quiet is 4
expect editor.settles_sent is 0
expect display.stale.200.300.1200.1700 is 0
expect display.dark.250.1160.1100.100 is 1015
expect editor.live_matches_render is 1
shot 03-erased-along
# The stroke eraser takes the third line away whole
action eraser-mode
expect settings.eraser_mode is 0
wait 2000
wait 30
eraser line 800 1440 820 1580 12
pen leave
wait 2000
wait 30
expect editor.strokes is 6
expect editor.erase_strokes is 2
expect display.stale.200.300.1200.1700 is 0
expect display.dark.250.1460.1100.100 is 0
expect editor.live_matches_render is 1
shot 04-stroke-erased
# White ink of the ballpoint along the fourth line, a pixel below it
action white
expect settings.white is 1
pen down 300 1801 0.6 0 0
pen move 400 1801 0.6 0 0
pen move 500 1801 0.6 0 0
pen move 600 1801 0.6 0 0
wait 30
pen move 700 1801 0.6 0 0
pen move 800 1801 0.6 0 0
pen move 900 1801 0.6 0 0
wait 30
pen move 1000 1801 0.6 0 0
pen move 1100 1801 0.6 0 0
pen move 1200 1801 0.6 0 0
pen move 1300 1801 0.6 0 0
expect display.stale.200.300.1200.1700 is 0
pen up 1300 1801
pen leave
wait 2000
wait 30
expect editor.strokes is 7
expect editor.erase_strokes is 3
expect editor.stroke_look is white ballpoint
expect editor.settles_sent is 0
expect display.stale.200.300.1200.1700 is 0
expect display.dark.250.1760.1100.100 is 1003
expect editor.live_matches_render is 1
shot 05-white-ink
# Undo and redo the white ink
pentap panel undo
wait 2000
wait 30
expect editor.strokes is 6
expect display.stale.200.300.1200.1700 is 0
expect display.dark.250.1760.1100.100 is 6014
expect editor.live_matches_render is 1
pentap panel redo
wait 2000
wait 30
expect editor.strokes is 7
expect display.stale.200.300.1200.1700 is 0
expect display.dark.250.1760.1100.100 is 1003
expect editor.live_matches_render is 1
shot 06-redone
# Closed (compacted in the background) and opened again: the same pixels
pentap panel library
wait 600
expect screen is library
waitbg
wait 300
tap library card.1
wait 700
expect screen is editor
expect editor.matches_closed is 1
expect editor.live_matches_render is 1
expect display.dark.250.860.1100.100 is 4224
expect display.dark.250.1160.1100.100 is 1015
expect display.dark.250.1460.1100.100 is 0
expect display.dark.250.1760.1100.100 is 1003
shot 07-reopened
quit
