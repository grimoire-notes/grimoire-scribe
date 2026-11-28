# rust-only
# generate: tests/home_fixture.py
# settings: status_size=0
# env: TZ=UTC
# A notebook's status bar on its gray band, drawn over the page (#337, the owner: "Status bar background drawn over
# any elements below but still drawable beneath, user dismisses to access that part of the screen"). Ink written in
# the bar's rows (a line inside them, a zigzag across their lower edge) goes into the page, and the band covers it:
# while the pen writes, the panel's band rows stay what the screen holds (no live frame reaches them), and after
# the settle the dark pixels of the strip are the bar's own, as before the ink. A tap on the bar hides it with one
# grays refresh of its strip alone (no flash of the whole screen) and the ink shows; a tap on the top strip brings it
# back the same way. A lasso drawn up through the bar's rows keeps its line under the band. Hidden, the notebook
# closed and opened again: the bar is back over the ink, which was kept, and a tap shows it. Sideways the band runs
# across the whole width. Then the band on Settings > Sleep & Clock > Status bar and on Files. The clock is held
# still, so the shots are the same on every run.
clock 1795772460
wait 3600
expect screen is library
tap library notebook.ideas
wait 800
expect screen is editor
expect status.visible is 1
expect status.zone is 0 0 1860 72
waitfor display.stale.0.0.1860.2480 is 0
# (the strip's dark pixels: the bar's own, the time and the date)
expect display.dark.0.0.1860.72 is 1696
shot 01-notebook-bar
expect editor.strokes is 37
# (and, the bar hidden, the page's own there before anything is written: none. The ink written below is counted as
# not none, since a stroke's anti-aliased edge differs by a few pixels from run to run)
tap 600 36
wait 400
expect status.visible is 0
expect display.dark.0.0.1860.72 is 0
tap 600 36
wait 400
expect status.visible is 1
expect display.dark.0.0.1860.72 is 1696
pen line 150 22 820 48 40 0.7 0.7
wait 200
pen down 900 24 0.7
pen move 960 70 0.7
pen move 1020 116 0.7
pen move 1080 70 0.7
pen move 1140 24 0.7
pen move 1200 70 0.7
pen move 1260 116 0.7
wait 100
expect display.pen_mode is 1
expect display.stale.0.0.1860.72 is 0
expect display.dark.0.0.1860.72 is 1696
pen move 1320 70 0.7
pen move 1380 24 0.7
pen move 1440 70 0.7
pen move 1500 116 0.7
pen up 1500 116
wait 300
pen leave
wait 2500
expect editor.strokes is 39
expect display.stale.0.0.1860.2480 is 0
expect display.dark.0.0.1860.72 is 1696
# (and the zigzag below the band, where nothing covers it)
expect display.dark.880.72.640.60 not 0
shot 02-bar-over-ink
expect display.presents not -1
tap 600 36
wait 400
expect status.visible is 0
expect status.zone is 0 0 0 0
expect display.presents is ui 0 0 1860 72
expect display.stale.0.0.1860.2480 is 0
expect display.dark.0.0.1860.72 not 0
shot 03-dismissed
tap 600 36
wait 400
expect status.visible is 1
expect display.presents is ui 0 0 1860 72
expect display.stale.0.0.1860.2480 is 0
expect display.dark.0.0.1860.72 is 1696
shot 04-bar-back
# a lasso up through the bar's rows, beside the ink, around nothing: its line stays under the band
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.select is armed
pen down 40 900
pen move 40 500
pen move 40 30
pen move 120 30
wait 100
expect display.stale.0.0.1860.72 is 0
expect display.dark.0.0.1860.72 is 1696
pen move 120 500
pen move 120 900
pen up 50 900
wait 300
pen leave
wait 2500
expect display.stale.0.0.1860.2480 is 0
expect display.dark.0.0.1860.72 is 1696
# hidden, the notebook left and opened again: the bar is back over the ink, which was kept
tap 600 36
wait 400
expect status.visible is 0
tap panel library
wait 600
expect screen is library
tap library notebook.ideas
wait 800
expect screen is editor
expect status.visible is 1
expect editor.strokes is 39
waitfor display.stale.0.0.1860.2480 is 0
expect display.dark.0.0.1860.72 is 1696
shot 05-reopened-bar-over-ink
tap 600 36
wait 400
expect status.visible is 0
expect display.dark.0.0.1860.72 not 0
shot 06-reopened-dismissed
tap 600 36
wait 400
expect status.visible is 1
# sideways: the band across the whole width, over the page and the desk beside it
rotate 1
wait 800
expect status.zone is 0 0 2480 72
expect status.visible is 1
shot 07-sideways-bar-over-ink
tap 600 36
wait 400
expect status.visible is 0
shot 08-sideways-dismissed
tap 600 36
wait 400
expect status.visible is 1
rotate 0
wait 500
# the band outside a notebook: Settings > Sleep & Clock > Status bar, and Files
tap panel library
wait 600
expect screen is library
tap library settings
wait 300
tap library settings.cat.2
wait 100
tap library status-bar
wait 300
shot 09-settings-status-bar
tap library display.done
wait 300
expect library.sheet is none
tap library app.files
wait 600
shot 10-files
quit
