# A Seyes notebook turned sideways on the Kindle's screen (issue #308): the whole page rests at 0.75, and
# the paper is drawn at the screen's scale, every line on whole pixels as wide and as gray as at 100% (the
# page shrunk with its lines gave each line its own weight: the inner ones a third heavier one to the next
# on the panel's 16 grays). Ink written upright and sideways shows over it, and a frame drawn in parts
# (the strokes written sideways) is the screen composed whole. Upright again, the page is at 100%.
# The status bar's clock is held still, since editor.screen_composed composes the bar with the page,
# and the pen leaves after each run of strokes, as a hand does.
# rust-only
clock 1700000000
wait 3600
tap library add
wait 200
tap library add.new-notebook
wait 200
expect library.sheet is new
tap library paper.20
wait 100
expect library.paper is 20
tap library create
wait 400
expect screen is editor
expect editor.paper is 20
expect editor.page_size is 1860x2480
expect editor.scale is 1.00
# Writing across the lines upright
pen curve 560 700 40 60 0.6 3
pen line 420 980 1500 1040 80 0.3 0.9
pen leave
wait 3000
expect editor.strokes is 2
shot 01-upright
rotate 1
wait 1500
expect size is 2480x1860
expect editor.scale is 0.75
expect editor.zoomed is 0
expect editor.screen_composed is 1
expect editor.layers is ok
shot 02-sideways
# Writing sideways, below 100%: the stroke's frames are drawn in parts
pen line 800 1100 1700 1150 60 0.4 0.8
pen curve 1200 1300 30 50 0.6 2
pen leave
wait 3000
expect editor.strokes is 4
expect editor.screen_composed is 1
expect editor.layers is ok
shot 03-written-sideways
rotate 0
wait 1500
expect size is 1860x2480
expect editor.scale is 1.00
expect editor.screen_composed is 1
shot 04-upright-again
pentap panel library
wait 500
expect screen is library
quit
