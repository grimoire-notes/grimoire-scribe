# settings: status_size=1
# env: GRIMOIRE_SIM_CLOCK=script
# rust-only
# The status bar's time is the platform's clock, the one a script's `clock` line sets and holds still
# (docs/SIM.md): while it stands nothing is refreshed for the clock, on the home screen, on a notebook's page or
# in the Pages view, and when the script moves it a minute on the home screen is drawn again once and a notebook
# refreshes the bar's strip alone, over its page and over its Pages view. Before, the bar read the machine's own
# clock behind the script's: the minute changed when the machine's did, and its refresh landed among the ones a
# scenario counts (issue #234: "ui 0 0 1860 78" beside a tile's own refresh).
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen: each in a wait of its own,
# since the second is done a pass of the main loop after the first. One wait of 3,600 ms made its time up in one
# step at its end on a slow machine, docs/SIM.md, did the first there, and left the second to land in the quiet
# stretch counted below.)
clock 1700000000
wait 1300
wait 2300
expect display.presents not -1
wait 2000
expect display.presents is
# a minute on: the home screen again, once
clock 1700000060
wait 200
expect display.presents is ui 0 0 1860 2480
wait 2000
expect display.presents is
# a notebook's page: the strip alone
tap library add
tap library add.new-notebook
wait 100
tap library create
wait 300
expect screen is editor
expect status.visible is 1
expect status.zone is 0 0 1860 78
wait 1500
expect display.presents not -1
wait 2000
expect display.presents is
clock 1700000120
wait 200
expect display.presents is ui 0 0 1860 78
wait 2000
expect display.presents is
# its Pages view: the strip alone too
tap panel pages
wait 500
expect editor.modal is pages
waitbg
wait 1500
expect display.presents not -1
wait 2000
expect display.presents is
clock 1700000180
wait 200
expect display.presents is ui 0 0 1860 78
wait 2000
expect display.presents is
quit
