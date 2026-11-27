# rust-only
# generate: tests/previews_fixture.py ghosts
# env: GRIMOIRE_SIM_PREVIEWS_CURRENT=1
# (The fixture's previews and card are as 2.0.0 or 2.0.1 left them, current for their page files: the knob makes
# stamps as before 2.3.0, with no renderer at their end, so they stay current here, #314.)
# A page entered from the Links bar comes from the bar's canvas, drawn from its strokes ahead of the tap, not drawn
# again as a page that opens is ("barghost": page 1 links to page 2, cleared under 2.0.0 or 2.0.1 with its drawing
# current, its Pages preview of the old ink; it opens on page 1). Staying on page 2 from the bar takes the old
# drawing away and makes the preview again from the page shown, and the Pages view shows it. Before, only a page
# that opened did: one stayed on from the bar (or through "Back to page N", the same step) kept the old preview.
# (A tile of a page with no ink still has one pixel darker than mid-gray since 2.3.0's darker rules, issue #314:
# Narrow rule's header crossing its margin. editor.pages.ink is 1 for such a tile, more where ink shows.)
wait 300
tap library notebook.barghost
expect editor.page is 1
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.1 is p2
tap 479 2218
wait 300
expect editor.bar is peek
expect editor.bar.peek is 2
expect editor.page is 1
tap 600 1100
wait 300
expect editor.bar is 0
expect editor.page is 2
expect editor.strokes is 0
expect editor.live_matches_render is 1
expect editor.preview_matches_render is 1
tap panel pages
expect editor.modal is pages
expect editor.pages.ink.2 is 1
shot 01-tile-2
quit
