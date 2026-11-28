# rust-only
# settings: shortcuts_enabled=1
# settings: status_size=0
# env: GRIMOIRE_WIFI_SCRIPT=tests/fixtures/wifi/helper.sh
# Settings > Wi-Fi's saved list scrolled by a finger (#337): its fast frames keep below the status bar, so the bar's
# gray band is never in one (black and white only, it showed dotted for the length of the drag when each frame was
# the whole screen); the lift's grays refresh is the whole screen, and the band is as it was.
clock 1700000000
wait 3600
tap library settings
wait 100
tap library wifi
wait 300
tap library wifi.radio
wait 800
expect wifi.saved is 15
tap library wifi.tab.saved
wait 300
waitfor display.stale.0.0.1860.2480 is 0
finger 1 down 930 1700
frame 20
wait 100
finger 1 move 930 1600
frame 20
wait 100
finger 1 move 930 1500
frame 20
wait 100
finger 1 move 930 1400
frame 20
wait 150
# mid-scroll: the list has moved on the panel, the band is as the screen holds it
expect display.presents not -1
expect display.stale.0.0.1860.72 is 0
expect display.dark.0.0.1860.72 not 0
shot 01-mid-scroll
finger 1 up 930 1400
frame 20
wait 1500
waitfor display.stale.0.0.1860.2480 is 0
shot 02-after-scroll
quit
