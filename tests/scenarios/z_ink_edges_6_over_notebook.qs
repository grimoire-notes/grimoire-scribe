# settings: status_mode=1
# Ink edges (2.3, #320) changed with a notebook open under Settings (the quick settings' Settings button): the open
# page is drawn again at once with the new setting, so it shows the strokes as the setting draws them, and an undo of
# a step from before the change draws its region again (the step's patch, pixels of the other setting, is gone:
# editor.undo_patched does not move), the page still its strokes drawn afresh. Back to Smooth likewise.
# rust-only
wait 3600
clock 1700000000
tap library add
tap library add.new-notebook
wait 100
tap library paper.1
wait 100
tap 1564 2254
wait 400
expect screen is editor
pentap panel tool-picker
pentap 1188 561
expect editor.tool is fineliner
pen line 300 500 1500 520 40 0.6 0.6
pen line 300 700 1500 720 40 0.6 0.6
pen line 300 900 1500 920 40 0.6 0.6
pen leave
wait 2000
expect editor.strokes is 3
pentap panel undo
pentap panel redo
wait 300
expect editor.undo_patched is 2
expect editor.live_matches_render is 1
shot 01-smooth
pen leave
wait 1100
touch down 930 40
touch move 930 140
wait 150
expect quick is 1
touch up 930 140
wait 100
tap panel quick.settings
wait 300
expect editor.modal is settings
tap library settings.cat.3
wait 100
tap library ink-edges
wait 100
tap library ink-edges.1
wait 300
expect settings.ink_edges is solid
expect ink_edges is solid
tap library done
wait 300
expect editor.modal is none
expect editor.live_matches_render is 1
expect display.stale.200.400.1400.600 is 0
shot 02-solid
pentap panel undo
wait 300
expect editor.strokes is 2
expect editor.undo_patched is 2
expect editor.live_matches_render is 1
pentap panel redo
wait 300
expect editor.strokes is 3
expect editor.undo_patched is 2
expect editor.live_matches_render is 1
pen leave
wait 1100
touch down 930 40
touch move 930 140
wait 150
touch up 930 140
wait 100
tap panel quick.settings
wait 300
tap library settings.cat.3
wait 100
tap library ink-edges
wait 100
tap library ink-edges.0
wait 300
expect ink_edges is smooth
tap library done
wait 300
expect editor.modal is none
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.undo_patched is 2
expect editor.live_matches_render is 1
quit
