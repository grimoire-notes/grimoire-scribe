# settings: status_mode=1
# Back on an 1860x2480 Scribe with what y_pagesize_3_near left. The notebook made on the 1920x2560
# screen is a little larger than this one: fitted, its whole page across this screen's width as
# before (at 100% part of it would be off the screen), its ink where it was written, new ink under
# the pen. The School notebook is at 100% with no offset again (the identity), with what was written
# at 100% on the larger screen at the same page points: only the view moved, never the strokes.
# root: y_pagesize_3_near
# settings: landscape_fit=0
# rust-only
wait 300
shot 01-library
expect size is 1860x2480
expect settings.pen.valid is 1
expect settings.pen.panel is 1860x2480
expect library.items is 3
# The notebook made on the 1920x2560 screen, with the pen
pentap library card.3
wait 500
shot 02-fitted
expect screen is editor
expect editor.page_size is 1920x2560
expect editor.scale is 0.97
expect editor.zoom is 1.00
expect editor.zoomed is 0
expect editor.strokes is 1
expect editor.stroke_start is 300 900
# Screen (300, 1200) is page (310, 1239)
pen line 300 1200 1500 1250 60 0.4 0.8
expect editor.stroke_start is 310 1239
wait 700
shot 03-written
expect editor.strokes is 2
expect editor.live_matches_render is 1
# Library with the pen. The School notebook: 100% and no offset, on page 2, left open there
pentap panel library
wait 500
expect screen is library
tap library card.1
wait 300
expect library.folder is School
pentap library card.1
wait 500
expect screen is editor
expect editor.page_size is 1860x2480
expect editor.view is 1.00 0 0
expect editor.page is 2
expect editor.strokes is 1
expect editor.stroke_start is 300 700
pentap panel previous
wait 500
shot 04-own-size
expect editor.page is 1
expect editor.strokes is 5
# the last of them, written pinched in on the larger screen, at the same page point
expect editor.stroke_start is 790 1090
expect editor.view is 1.00 0 0
expect editor.live_matches_render is 1
pentap panel library
wait 500
expect screen is library
quit
