# settings: status_mode=1
# settings: status_size=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitpanel .
# An app's passes refresh all they change, the status bar too (issue #272), with Black app of tests/fixtures/kitpanel
# (rust/tests/kitfix: `make kit-fixtures`), which shows the status bar and fills a black square 100 px a side about
# its last touch. The bar's new minute is a pass of the whole app: drawn while a finger is down, it shows where the
# finger came down (app_pointer kept the place), and refreshes that with the bar, so the tap's own pass at the lift
# has nothing left to send and the panel holds what the screen does. A tap in the batch that brings a new minute: the
# tap's pass draws the bar's new minute too, and refreshes it with the square.
clock 1700000000
wait 3600
tap library apps
wait 300
tap library app.test.kit.black
wait 400
expect library.app.open is 1
waitfor display.stale.0.0.1860.2480 is 0
expect display.presents not -1
touch down 900 1400
wait 100
clock 1700000041
wait 100
# (one refresh: the bar's minute and the square's two places)
expect display.presents is ui 104 29 846 1421
touch up 900 1400
wait 1000
expect library.app.status has t01
expect display.presents is
expect display.stale.0.0.1860.2480 is 0
clock 1700000101
tap 600 1000
wait 300
expect library.app.status has t02
expect display.presents is ui 104 29 846 1421
expect display.stale.0.0.1860.2480 is 0
quit
