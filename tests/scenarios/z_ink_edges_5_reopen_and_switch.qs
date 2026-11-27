# settings: status_mode=1
# settings: ink_edges=solid
# env: GRIMOIRE_KEEP_RENDER_MS=0
# env: GRIMOIRE_PAGE_VIEWS=0
# Ink edges (2.3, #320): a page's pictures follow the setting. Written with Solid and left, the notebook opens again
# from the drawing kept as it was left (every page keeps one here: GRIMOIRE_KEEP_RENDER_MS=0), the same pixels as
# drawn live, and its card is the page drawn with Solid. Switched to Smooth, the next open draws the page whole (the
# drawing kept under Solid is not read) and keeps a Smooth one; the card on file is still Solid's then, and follows
# when the notebook is left; back to Solid likewise. The page is the strokes drawn afresh at every step.
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
pentap 872 561
expect editor.tool is ballpoint
pen line 300 500 1500 520 40 0.6 0.6
pentap panel tool-picker
pentap 872 395
expect editor.tool is pencil
pen line 300 700 1500 720 40 0.6 0.6
pen leave
wait 2000
expect editor.live_matches_render is 1
pentap panel library
wait 600
expect screen is library
tap library card.1
wait 700
expect screen is editor
expect editor.page_kept is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render is 1
expect editor.card_matches_render is 1
shot 01-reopened-solid
# Smooth: the drawing kept under Solid is not read; the page is drawn whole and keeps its drawing at once (it took
# longer than GRIMOIRE_KEEP_RENDER_MS); the card is drawn again as the notebook is left
pentap panel library
wait 600
tap library settings
wait 100
tap library settings.cat.3
wait 100
tap library ink-edges
wait 100
tap library ink-edges.0
wait 100
expect settings.ink_edges is smooth
tap library done
wait 100
tap library card.1
wait 700
expect screen is editor
expect editor.page_kept is 0
expect editor.live_matches_render is 1
expect editor.kept_matches_render is 1
expect editor.card_matches_render is 0
shot 02-reopened-smooth
pentap panel library
wait 600
tap library card.1
wait 700
expect editor.page_kept is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render is 1
expect editor.card_matches_render is 1
# Solid again: likewise
pentap panel library
wait 600
tap library settings
wait 100
tap library settings.cat.3
wait 100
tap library ink-edges
wait 100
tap library ink-edges.1
wait 100
expect settings.ink_edges is solid
tap library done
wait 100
tap library card.1
wait 700
expect editor.page_kept is 0
expect editor.live_matches_render is 1
expect editor.card_matches_render is 0
pentap panel library
wait 600
tap library card.1
wait 700
expect editor.page_kept is 1
expect editor.kept_matches_render is 1
expect editor.card_matches_render is 1
quit
