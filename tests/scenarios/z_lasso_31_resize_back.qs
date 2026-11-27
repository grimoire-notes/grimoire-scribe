# settings: status_mode=1
# settings: mirror_pdf=0
# settings: eraser_mode=1
# rust-only
# Issue #204: a resize and the opposite one give the page back, as they did when a resize kept the widths. A word
# one pass of the default precise eraser (8 mm) took away with a few pixels to spare, lassoed with a word that
# shows: made twice as large, its erasing is drawn at 12 mm, the most the brush draws and a page file keeps, so
# the word's loops show past it; made as large as it was again, the page shows nothing of it. The page in memory
# keeps each width as scaled (lasso.rs lasso_scale_widths), so this holds in the same selection and, while the
# notebook stays open, after Done too. A page file keeps 12 mm (store.rs page_serialize): after the notebook is
# opened again a resize back starts from 12 mm, the erasing comes back narrower than it was and the word's loops
# show (LASSO-DESIGN.md row 10). A width clamped at each resize lost its value in the same selection already.
# display.dark counts what the panel shows dark in a place.
# (the clock corner held still: its minute's refresh would land among the counted ones)
# (Since 2.3.0 the paper's rules are darker than mid-gray, issue #314, so display.dark counts them: a place with no
# ink counts its rules' pixels, two rows of each rule across it, and "not N" there says ink shows beside them.)
clock 1700000000
wait 300
tap library add
tap library add.new-notebook
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect editor.tool is pencil
pen curve 400 700 40 60 0.6 4
pen curve 400 900 40 60 0.6 4
wait 300
eraser line 300 900 960 900 40
waitfor display.stale.300.600.1000.800 is 0
expect editor.strokes is 3
expect editor.page_sig_keep is kept
expect display.dark.360.850.540.100 is 1080
expect display.dark.360.650.540.100 not 2160
# twice as large and back in the same selection, then Done
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 280 600
pen move 1000 600
pen move 1000 1000
pen move 280 1000
pen up 280 620
wait 300
expect editor.selected is 2
expect editor.sel_box is 360 645 536 311
pen down 896 956
pen move 1030 1034
wait 150
pen move 1164 1112
wait 150
pen move 1298 1189
wait 150
pen move 1432 1267
wait 150
pen up 1432 1267
wait 600
expect editor.sel_box is 360 645 1072 622
expect editor.live_matches_render is 1
pen down 1432 1267
pen move 1298 1189
wait 150
pen move 1164 1112
wait 150
pen move 1030 1034
wait 150
pen move 896 956
wait 150
pen up 896 956
wait 600
expect editor.sel_box is 360 645 536 311
expect editor.live_matches_render is 1
tap panel select.done
wait 300
waitfor display.stale.300.600.1500.1300 is 0
expect display.dark.360.850.540.100 is 1080
# (the word still shows nothing: a lasso round it alone selects nothing)
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
waitfor toast is
pen down 330 840
pen move 960 840
pen move 960 975
pen move 330 975
pen up 330 850
wait 300
expect editor.selected is 0
expect toast is Nothing to select.
pentap panel undo
wait 400
pentap panel undo
wait 400
expect editor.strokes is 3
expect editor.page_sig_same is 1
# twice as large, Done: the word's loops show past the 12 mm erasing; lassoed again and made as large as it was,
# Done: nothing of it
waitfor toast is
pen down 280 600
pen move 1000 600
pen move 1000 1000
pen move 280 1000
pen up 280 620
wait 300
expect editor.selected is 2
pen down 896 956
pen move 1030 1034
wait 150
pen move 1164 1112
wait 150
pen move 1298 1189
wait 150
pen move 1432 1267
wait 150
pen up 1432 1267
wait 600
expect editor.sel_box is 360 645 1072 622
tap panel select.done
wait 300
waitfor display.stale.300.600.1500.1300 is 0
expect display.dark.356.1060.1050.190 not 6300
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 280 600
pen move 1500 600
pen move 1500 1330
pen move 280 1330
pen up 280 620
wait 300
expect editor.selected is 2
expect editor.sel_box is 363 648 1066 615
pen down 1432 1267
pen move 1298 1189
wait 150
pen move 1164 1112
wait 150
pen move 1030 1034
wait 150
pen move 896 956
wait 150
pen up 896 956
wait 600
expect editor.sel_box is 363 648 533 308
tap panel select.done
wait 300
waitfor display.stale.300.600.1500.1300 is 0
expect display.dark.360.850.540.100 is 1080
expect display.dark.360.650.540.100 not 2160
# twice as large, Done, and the notebook closed and opened again: its file keeps the erasing at 12 mm, so made as
# large as it was, the word's loops show
pentap panel undo
wait 400
pentap panel undo
wait 400
expect editor.strokes is 3
expect editor.page_sig_same is 1
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 280 600
pen move 1000 600
pen move 1000 1000
pen move 280 1000
pen up 280 620
wait 300
expect editor.selected is 2
pen down 896 956
pen move 1030 1034
wait 150
pen move 1164 1112
wait 150
pen move 1298 1189
wait 150
pen move 1432 1267
wait 150
pen up 1432 1267
wait 600
tap panel select.done
wait 300
pentap panel library
wait 500
expect screen is library
tap library card.1
wait 600
expect screen is editor
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 280 600
pen move 1500 600
pen move 1500 1330
pen move 280 1330
pen up 280 620
wait 300
expect editor.selected is 2
expect editor.sel_box is 363 648 1066 615
pen down 1432 1267
pen move 1298 1189
wait 150
pen move 1164 1112
wait 150
pen move 1030 1034
wait 150
pen move 896 956
wait 150
pen up 896 956
wait 600
expect editor.sel_box is 363 648 533 308
tap panel select.done
wait 300
waitfor display.stale.300.600.1500.1300 is 0
expect display.dark.360.850.540.100 not 1080
quit
