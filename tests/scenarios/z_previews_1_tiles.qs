# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_SIM_DENSE_DRAW_MS=4000 GRIMOIRE_SIM_CORES=1
# The Pages view's previews come one tile at a time (2.0.0-beta.2): each tile is drawn as its preview lands
# (the job tells of each one), the light pages first, while a dense page (page 1, 1.3 MB, as slow to draw as
# a Kindle draws one: GRIMOIRE_SIM_DENSE_DRAW_MS) is still being drawn. A tile still waiting shows the loading
# mark (an hourglass) until its preview replaces it, and that tile alone is refreshed then.
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen)
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap library notebook.previews
expect editor.page is 2
tap panel pages
expect editor.modal is pages
# the light pages' previews have come, page 2's from its canvas as the view opened; page 1 is still drawn
# (waited for, not a time they take on a quiet machine: the dense page's own 4 s, which the job spends last, keep
# the tiles so until they are counted from)
waitfor editor.pages.shows is lpppppppp
expect bg.busy is 1
shot 01-loading
expect display.presents not -1
waitbg 30000
# the dense page's preview in its tile, that tile alone refreshed (a GL16, no flash)
waitfor editor.pages.shows is ppppppppp
expect display.presents is ui 24 292 588 648
shot 02-ready
quit
