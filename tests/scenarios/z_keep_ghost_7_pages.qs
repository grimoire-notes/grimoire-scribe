# rust-only
# generate: tests/previews_fixture.py ghosts
# env: GRIMOIRE_SIM_PREVIEWS_CURRENT=1
# (The fixture's previews and card are as 2.0.0 or 2.0.1 left them, current for their page files: the knob keeps
# them so as the app starts, where 2.3.0 puts every preview an earlier renderer drew out of date, #314.)
# A cleared page's old picture that 2.0.0 or 2.0.1 kept (their drawing current for the page file, the Pages preview
# and the Library card made from it: "ghost"), seen in the Pages view before the page is opened. Opened, the page is
# drawn from its strokes, the old drawing goes and the preview is made again from the page drawn, its page file as it
# was: the Pages view decodes it again (a tile is of its page file and of its preview file). Before, the view kept the
# tile it decoded of the old preview by its page file alone, at every open of the view and across the notebook
# opened again, until Grimoire was started again.
# (A tile of a page with no ink still has one pixel darker than mid-gray since 2.3.0's darker rules, issue #314:
# Narrow rule's header crossing its margin. editor.pages.ink is 1 for such a tile, more where ink shows.)
wait 300
tap library notebook.ghost
expect editor.page is 2
tap panel pages
expect editor.modal is pages
expect editor.pages.ink.1 not none
expect editor.pages.ink.1 not 1
shot 01-old-tile
tap panel pages.page.1
tap panel pages.page.1
wait 300
expect editor.modal is none
expect editor.page is 1
expect editor.page_kept is 0
expect editor.live_matches_render is 1
expect editor.preview_matches_render is 1
tap panel pages
expect editor.modal is pages
expect editor.pages.ink.1 is 1
shot 02-tile-again
tap panel pages.page.2
tap panel pages.page.2
wait 300
expect editor.page is 2
tap panel library
wait 600
expect screen is library
tap library notebook.ghost
expect editor.page is 2
tap panel pages
expect editor.modal is pages
expect editor.pages.ink.1 is 1
quit
