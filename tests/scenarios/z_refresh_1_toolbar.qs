# settings: status_mode=1
# rust-only
# Issue #11, the toolbar: a drag draws the panel by fast (DU) frames, in black and white, and its release
# redraws every place they drew it in grays, without a flash (a Ui refresh of that trail alone): a drag that
# went round leaves no page between its ends dithered (it did: only where it began and ended was drawn
# again). Each drag counts the panel's area (5.1 % of the screen) toward DU wear, as the eraser's strokes
# count theirs, so the fifth one tips it past a quarter and the way they went is cleaned with one Clean
# refresh at the next pause. display.stale counts what the panel shows other than the app's screen, over the
# page away from the clock corner.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap library add
tap library add.new-notebook
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect editor.panel_rect is 1704 220 128 1842
# (the launch repaints the whole screen 1.2 s and 3.5 s after it starts: past both before counting)
wait 3700
expect display.presents not -1
expect editor.wear is 0
expect display.stale.800.300.400.1800 is 0
# Round the page: left along the top, down, back right lower down. Each frame a fast refresh.
touch down 1737 260
wait 100
touch move 1500 260
wait 100
touch move 1200 260
wait 100
touch move 900 300
wait 100
touch move 900 500
wait 100
touch move 1200 700
wait 100
touch move 1500 700
wait 100
touch move 1737 700
wait 100
# the page's gray rules where the panel was, drawn back by DU: dithered on the panel for now
expect display.stale.800.300.400.1800 not 0
expect display.presents not -1
touch up 1737 700
wait 300
expect editor.panel_rect is 1704 638 128 1842
# one grayscale refresh of the whole trail, where it began, went round and ended: no flash
expect display.presents is ui 867 220 965 2260
expect display.stale.800.300.400.1800 is 0
expect display.stale.0.300.1860.2100 is 0
expect editor.wear is 51
expect editor.layers is ok
# A drag the pen's coming near cancels: grayscale at once, and its wear counted as any drag's.
pen leave
wait 400
expect display.presents not -1
finger 1 down 1737 700
frame 100
finger 1 move 1500 700
frame 100
finger 1 move 1100 700
frame 100
finger 1 move 1100 900
frame 100
pen hover 300 300
wait 300
expect editor.panel_rect is 1067 638 128 1842
expect display.presents has ui
expect display.presents not has clean
expect editor.wear is 102
expect display.stale.1000.700.800.1900 is 0
finger 1 up 1100 900
frame
pen leave
wait 400
expect display.presents not has clean
expect editor.wear is 102
expect display.stale.0.300.1860.2100 is 0
expect editor.layers is ok
# Three drags more, back and forth: the fifth tips the wear past a quarter of the screen, and the pause
# after it cleans the way they all went at once.
touch down 1100 700
wait 100
touch move 1500 700
wait 100
touch up 1500 700
wait 300
expect editor.wear is 153
touch down 1500 700
wait 100
touch move 1100 700
wait 100
touch up 1100 700
wait 300
expect editor.wear is 204
expect display.presents not has clean
touch down 1100 700
wait 100
touch move 1500 700
wait 100
touch up 1500 700
wait 600
expect display.presents has clean
expect editor.wear is 0
expect display.stale.0.300.1860.2100 is 0
expect editor.layers is ok
