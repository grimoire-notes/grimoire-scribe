# settings: status_mode=1
# The data b_library made on an 1860x2480 Scribe, opened on a screen a little larger (1920x2560: the
# notebook's page is 3.1% smaller on both axes). Its page rests at 100%, centred, with 30 px of desk
# at the sides and 40 at the top and bottom, where it was fitted at 1.03 before (no zoom chip either
# way). Ink lands at the page point under the pen, and the page canvas is pixel for pixel what a
# redraw gives; a pinch zooms in (ink lands right there too), and closing the fingers past 100%
# comes back to rest. Sideways the page's width fits the screen's, as before, and upright again it
# rests at 100%. A new page gets the notebook's size and rests the same. A notebook made here gets
# the screen's size and shows at 100% with no offset, as a notebook does on its own Scribe.
# root: b_library
# env: GRIMOIRE_SIM_SIZE=1920x2560
# settings: landscape_fit=0
# rust-only
wait 300
shot 01-library
expect size is 1920x2560
expect settings.pen.valid is 1
expect settings.pen.panel is 1920x2560
# The School folder with the pen, its notebook (sheet music, two strokes) with a finger
pentap 336 622
wait 300
expect library.folder is School
tap 336 622
wait 500
shot 02-at-100
expect screen is editor
expect editor.page_size is 1860x2480
expect editor.scale is 1.00
expect editor.zoom is 1.00
expect editor.view is 1.00 -30 -40
expect editor.zoomed is 0
expect editor.paper is 3
expect editor.strokes is 2
expect editor.screen_composed is 1
# b_library's last stroke, where it was written on the 1860x2480 screen
expect editor.stroke_start is 300 520
# Ink at 100%: screen (330, 1540) is page (300, 1500), a whole-pixel shift and nothing scaled
pen line 330 1540 1530 1600 80 0.3 0.9
expect editor.stroke_start is 300 1500
pen curve 450 1890 40 60 0.6 3
wait 700
shot 03-written
expect editor.strokes is 4
expect editor.live_matches_render is 1
wait 1500
expect editor.screen_composed is 1
expect display.stale.330.1500.1260.500 is 0
# Two fingers spread: zoomed in; screen (700, 1000) is page (790, 1090) there
pinch 960 1280 400 900 6
wait 500
shot 04-zoomed
expect editor.zoomed is 1
expect editor.scale is 1.86
expect editor.view is 1.86 414 553
pen line 700 1000 1300 1050 40 0.4 0.8
expect editor.stroke_start is 790 1090
wait 700
expect editor.strokes is 5
expect editor.live_matches_render is 1
# Closing them past 100%: held at 100%, centred on whole pixels, at rest again
pinch 960 1280 900 300 8
wait 500
shot 05-back-at-rest
expect editor.zoomed is 0
expect editor.scale is 1.00
expect editor.view is 1.00 -30 -40
# Sideways: the page's width fits the screen's, from its top (2560 / 1860), as before
rotate 1
wait 500
shot 06-sideways
expect size is 2560x1920
expect editor.page_size is 1860x2480
expect editor.scale is 1.38
expect editor.zoom is 1.00
expect editor.zoomed is 0
rotate 0
wait 500
expect size is 1920x2560
expect editor.scale is 1.00
expect editor.view is 1.00 -30 -40
expect editor.zoomed is 0
# A new page (the panel's + with the pen): the notebook's size, at 100% and centred too
pentap panel next
wait 500
expect editor.page is 2
expect editor.pages is 2
expect editor.page_size is 1860x2480
expect editor.view is 1.00 -30 -40
pen line 330 740 1530 800 60 0.4 0.8
expect editor.stroke_start is 300 700
wait 700
shot 07-new-page
expect editor.strokes is 1
expect editor.live_matches_render is 1
# Library with the pen; back to the top level with a finger
pentap panel library
wait 500
expect screen is library
tap library back
wait 300
expect library.folder is
# A notebook made here: this screen's size, 100% and no offset
tap library new
wait 300
expect library.sheet is new
tap library create
wait 500
expect screen is editor
expect editor.page_size is 1920x2560
expect editor.scale is 1.00
expect editor.view is 1.00 0 0
expect editor.zoomed is 0
pen line 300 900 1600 900 60 0.5 0.9
expect editor.stroke_start is 300 900
wait 700
shot 08-made-here
expect editor.strokes is 1
pentap panel library
wait 500
shot 09-library
expect screen is library
expect library.items is 3
quit
