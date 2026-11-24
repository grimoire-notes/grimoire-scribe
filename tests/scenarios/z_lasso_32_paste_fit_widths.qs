# rust-only
# generate: tests/lasso_more_fixture.py
# settings: eraser_mode=1
# A paste scaled down to fit a smaller page scales the widths with the points, as a resize does (the owner's ruling
# of 2026-11-23, after issue #204: lasso.rs lasso_scale_widths, called from sel_paste with paste_spot's scale):
# black ink, white ink and the erasing carried with the ink alike, kept as scaled in memory and drawn and stored
# within 0.1 to 12 mm. Before, the widths stayed in mm: the group's lines came out as wide as they were on the
# larger page and its erasing reached past where it had cut. Here a group as wide as a 1986 px page goes into a
# 1860 px one, 0.9366 times: the factor the larger page is shown at on this panel, so with the widths scaled the
# pasted group looks on the screen as it did there. editor.stroke_screen_mm reads the newest stroke's width as the
# screen shows it (the precise eraser's: its setting, drawn on the larger page at 8.00).
wait 300
tap library notebook.lasso-big
wait 1000
expect screen is editor
pentap panel next
wait 300
expect editor.page is 2
expect editor.strokes is 0
# black ink: a marker line across the page, copied, pasted into the smaller page
tap panel tool-picker
wait 100
tap panel picker.tool:marker
wait 100
expect editor.tool is marker
pen line 15 1200 1845 1200 80 0.6 0.6
wait 200
expect editor.stroke_screen_mm is 0.94
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 5 1100
pen move 930 1100
pen move 1855 1100
pen move 1855 1300
pen move 930 1300
pen move 5 1300
pen up 5 1110
wait 300
expect editor.select is selected
expect editor.selected is 1
expect editor.sel_box is 0 1263 1986 37
tap panel select.copy
wait 100
tap panel select.done
wait 100
pentap panel library
wait 500
expect screen is library
tap library notebook.lasso-small
wait 1000
expect screen is editor
expect editor.strokes is 0
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
tap panel select.paste
wait 300
expect editor.strokes is 1
expect editor.sel_box is 0 1222 1860 35
expect editor.stroke_screen_mm is 0.94
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.strokes is 0
tap panel select.done
wait 100
pentap panel library
wait 500
# white ink: a cut across the line with the white fountain pen, copied with the line
tap library notebook.lasso-big
wait 1000
expect screen is editor
expect editor.page is 2
expect editor.strokes is 1
tap panel tool-picker
wait 100
tap panel picker.tool:fountain
wait 100
tap panel tool-picker
wait 100
tap panel picker.ink.1
wait 100
tap 100 2400
wait 100
expect settings.white is 1
pen line 600 1150 600 1250 20 1.0 1.0
pen leave
wait 300
expect editor.stroke_screen_mm is 0.94
tap panel tool-picker
wait 100
tap panel picker.ink.0
wait 100
tap 100 2400
wait 100
expect settings.white is 0
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 5 1100
pen move 930 1100
pen move 1855 1100
pen move 1855 1300
pen move 930 1300
pen move 5 1300
pen up 5 1110
wait 300
expect editor.selected is 1
tap panel select.copy
wait 100
tap panel select.done
wait 100
pentap panel library
wait 500
tap library notebook.lasso-small
wait 1000
expect screen is editor
expect editor.strokes is 0
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
tap panel select.paste
wait 300
expect editor.strokes is 2
expect editor.stroke_screen_mm is 0.94
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.strokes is 0
tap panel select.done
wait 100
pentap panel library
wait 500
# erasing: a cut across the line with the precise eraser, copied with the line and the white ink
tap library notebook.lasso-big
wait 1000
expect screen is editor
expect editor.page is 2
expect editor.strokes is 2
tap panel tool-picker
wait 100
tap panel picker.tool:marker
wait 100
eraser line 1200 1140 1200 1260 12
wait 300
expect editor.strokes is 3
expect editor.stroke_screen_mm is 8.00
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 5 1100
pen move 930 1100
pen move 1855 1100
pen move 1855 1300
pen move 930 1300
pen move 5 1300
pen up 5 1110
wait 300
expect editor.selected is 1
tap panel select.copy
wait 100
tap panel select.done
wait 100
pentap panel library
wait 500
tap library notebook.lasso-small
wait 1000
expect screen is editor
expect editor.strokes is 0
expect editor.page_sig_keep is kept
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
tap panel select.paste
wait 300
expect editor.strokes is 3
expect editor.sel_box is 0 1222 1860 35
expect editor.stroke_screen_mm is 8.00
expect editor.live_matches_render is 1
# undone, the page is as it was; redone, the same widths
pentap panel undo
wait 100
expect editor.strokes is 0
expect editor.page_sig_same is 1
pentap panel redo
wait 100
expect editor.strokes is 3
expect editor.stroke_screen_mm is 8.00
expect editor.live_matches_render is 1
tap panel select.done
wait 300
waitfor display.stale.1000.1150.400.150 is 0
# the cut is 0.9366 times as wide as it was: the line shows up to 2 px from where the cut would end with the
# eraser's width kept (columns 1154 and 1155, 1244 and 1245: erased with the width kept), and nothing in the cut
expect display.dark.1160.1222.80.35 is 0
expect display.dark.1154.1222.2.35 not 0
expect display.dark.1244.1222.2.35 not 0
expect display.dark.1100.1222.40.35 not 0
expect display.dark.1270.1222.40.35 not 0
shot 01-pasted
quit
