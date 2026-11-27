# rust-only
# generate: tests/previews_fixture.py
# settings: status_mode=1
# Ink edges (2.3, #320) changed over a notebook open on a dense page (its file 1280 KB): the page is drawn again with
# the new setting at once, and as it takes seconds on the Kindle, "Loading page..." goes up first, as when the page
# opens (the drawing kept under Smooth is not read under Solid). The page is the strokes drawn afresh.
clock 1700000000
wait 3600
tap library notebook.dense
wait 300
expect editor.page is 1
expect editor.loading_toasts is 1
expect toast is
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
expect editor.loading_toasts is 2
expect editor.page_kept is 0
expect toast is
tap library done
wait 300
expect editor.modal is none
expect editor.live_matches_render is 1
quit
