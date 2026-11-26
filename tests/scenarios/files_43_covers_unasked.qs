# settings: status_mode=1
# settings: files_card=1
# settings: files_view=covers
# storage: 1
# generate: tests/files_place_tree.py
# copy: tests/fixtures/pdf/planner documents/Covers
# copy: tests/fixtures/pdf/junk documents/Covers
# copy: tests/fixtures/photos documents/Covers
# copy: tests/fixtures/fonts/good documents/Covers
# env: GRIMOIRE_SIM_CLOCK=script
# env: GRIMOIRE_BG_DELAY_MS=150
# Files' Covers view and the platform (issue #280): the refresh that draws the tiles whose previews came is one
# nobody asked for, and app_flush says so to the platform (display_unasked, the runner's display.unasked), which may
# leave out a cleaning flash of its own for it. The taps that open Files, a folder and the Covers folder, and the
# Back that leaves it, are refreshes something asked for, and go without. As in files_11_covers_refresh.qs, eight
# files are made one after another while the app's clock stands still, and all eight tiles go out as one ui refresh
# of their union. (The clock corner held still: its minute's refresh would land among the counted ones.)
# rust-only
clock 1700000000
wait 3600
expect display.unasked not -1
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Covers
wait 400
expect files.tiles is llllllll
expect display.unasked is 0
expect display.presents not -1
waitbg
wait 60
expect files.tiles is pipppppp
expect files.making is 0
expect display.presents is ui 46 356 1766 1937
expect display.unasked is 1
shot 01-made
tap library files.back
wait 300
expect display.unasked is 0
quit
