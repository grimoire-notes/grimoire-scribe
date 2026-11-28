# settings: status_size=1
# settings: ink_edges=smooth
# env: GRIMOIRE_SIM_CLOCK=script
# env: TZ=UTC
# rust-only
# A notebook's status bar waits for the pen (issue #239), as the clock corner it replaced did. A grayscale
# refresh ends the driver's pen mode and waits for the pen's last frame, so the refresh of the bar's strip for a
# new minute ("ui 0 0 1860 78", tests/scenarios/y_status_bar_clock.qs) is held back while the pen is near
# (`pen hover` until `pen leave`), a stroke or a drag is in progress or `display.pen_mode` is 1 (status.rs
# status_busy, which editor_tick asks). When the pen has left it comes once, with the minute that is current
# then; a finger coming down takes a "near" back too, as it did for the corner. The same over the page, the Pages
# view and the Graph. Before, the strip was refreshed under a hovering pen and between two moves of a stroke.
# While a minute waits, a stroke's own refreshes that reach the strip show the time the strip holds, never part
# of the next minute (status.rs status_bar_time: the page draws part of the strip with the time the whole strip
# was last drawn with).
# The time on the panel is read as the dark pixels of the strip's left end (the time and the date, 2023-11-14
# UTC from 22:13 on: each minute here has a count of its own beside the one before it).
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen)
wait 3600
clock 1700000000
wait 300
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
expect display.dark.45.0.400.60 is 1604
# no pen about: a minute on, the strip alone, at once
clock 1700000060
wait 200
expect display.presents is ui 0 0 1860 78
expect display.dark.45.0.400.60 is 1580
wait 2000
expect display.presents is
# the pen near: a minute on and then another, and nothing is refreshed however long it stays
pen hover 900 900
clock 1700000120
wait 300
expect display.presents is
wait 2000
expect display.presents is
expect display.dark.45.0.400.60 is 1580
clock 1700000180
wait 300
expect display.presents is
# it leaves: the strip once, with the minute that is current now (22:16), and nothing after it
pen leave
wait 300
expect display.presents is ui 0 0 1860 78
expect display.dark.45.0.400.60 is 1636
wait 2000
expect display.presents is
# a stroke in progress: the same, and the stroke's own refreshes are not held for it
pen down 400 600 0.6
pen move 460 640 0.6
wait 100
expect display.pen_mode is 1
expect display.presents not -1
clock 1700000240
wait 300
expect display.presents is
pen move 520 660 0.6
wait 100
expect display.presents is fast 455 635 72 32
pen up 520 660
wait 300
expect display.pen_mode is 0
expect display.presents is
# lifted and still near: the stroke settles, and the bar still waits
wait 2000
expect display.presents is ui 383 220 1449 1734
expect display.dark.45.0.400.60 is 1636
pen leave
wait 300
expect display.presents is ui 0 0 1860 78
expect display.dark.45.0.400.60 is 1544
wait 2000
expect display.presents is
# a pen whose leaving was never told: a finger coming down is not the pen's hand, and the refresh comes
pen hover 900 900
clock 1700000300
wait 300
expect display.presents is
tap 930 1700
wait 300
expect display.presents is ui 0 0 1860 78
expect display.dark.45.0.400.60 is 1644
wait 2000
expect display.presents is
# a stroke beside the time while a minute waits (down the left of it and along under it, no ink on it, all in the
# bar's rows): its settle reaches the time, and the time it shows is still the strip's, whole. Its live frame sends
# nothing: since #337 the bar is drawn over the page on its band, and no fast refresh reaches the band's rows (it
# was `fast 25 3 222 74`)
pen hover 900 900
clock 1700000360
wait 300
expect display.presents is
pen down 30 8 0.6
pen move 30 40 0.6
pen move 30 70 0.6
pen move 130 70 0.6
pen move 240 70 0.6
wait 100
expect display.presents is
pen up 240 70
wait 2000
expect display.presents is ui 13 0 246 89
expect display.dark.45.0.400.60 is 1644
pen leave
wait 300
expect display.presents is ui 0 0 1860 78
expect display.dark.45.0.400.60 is 1636
wait 2000
expect display.presents is
# the Pages view, where the pen drags pages: the same
tap panel pages
wait 500
expect editor.modal is pages
waitbg
wait 1500
expect display.presents not -1
wait 2000
expect display.presents is
pen hover 900 1200
clock 1700000420
wait 300
expect display.presents is
expect display.dark.45.0.400.60 is 1636
pen leave
wait 300
expect display.presents is ui 0 0 1860 78
expect display.dark.45.0.400.60 is 1716
wait 2000
expect display.presents is
# the Graph, which the pen drags: the same
tap panel pages.graph
wait 500
expect editor.modal is map
wait 3000
expect display.presents not -1
wait 2000
expect display.presents is
pen hover 900 1200
clock 1700000480
wait 300
expect display.presents is
pen leave
wait 300
expect display.presents is ui 0 0 1860 78
expect display.dark.45.0.400.60 is 1604
wait 2000
expect display.presents is
quit
