# settings: status_mode=1
# settings: ink_edges=solid
# Ink edges at Solid (2.3, #320) on a page a little smaller than the screen (issue #303): b_library's 1860x2480
# notebook on a screen of 1920x2560 rests at 100%, centred, with 30 px of desk at each side. The view copies the
# page there, pixel for pixel, so the Solid pass of the scaled views has nothing to make again: a solid stroke down
# the page's first columns leaves the desk beside it as it was (display.dark 0 over the desk's band; run there, the
# pass would draw the stroke's black a pixel into the desk), and the screen is the page and its layers.
# root: b_library
# env: GRIMOIRE_SIM_SIZE=1920x2560
# settings: landscape_fit=0
# rust-only
wait 3600
clock 1700000000
expect size is 1920x2560
pentap library card.1
wait 300
expect library.folder is School
tap library card.1
wait 500
expect screen is editor
expect editor.page_size is 1860x2480
expect editor.view is 1.00 -30 -40
expect editor.zoomed is 0
expect ink_edges is solid
action next-tool
expect editor.tool is fineliner
pen leave
wait 2000
expect display.dark.0.200.30.2200 is 0
# Down the page's left edge: screen column 31 is the page's second
pen line 31 1200 33 1700 30 0.7 0.7
pen leave
wait 2000
expect editor.strokes is 3
expect display.stale.0.200.400.2200 is 0
expect display.dark.0.200.30.2200 is 0
expect display.dark.30.1200.4.500 not 0
expect editor.live_matches_render is 1
expect editor.screen_composed is 1
shot 01-left-edge
quit
