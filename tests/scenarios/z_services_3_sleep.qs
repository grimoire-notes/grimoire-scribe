# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: sleep_mode=2
# settings: sleep_minutes=1
# copy: tests/fixtures/gestures/actions .
# env: GRIMOIRE_SIM_NO_SERVICES=sleep GRIMOIRE_SLEEP_MINUTE_MS=1000 GRIMOIRE_CHECK_FIT=1
# A platform whose system owns sleep (light.rs, Services, sleep): its own timer and sleep picture, and no
# sleep on request. Sleep after asks nothing; the sleep screen is the page whatever settings.ini says (the
# Kindle's own screensaver here); Sleep settings has one switch, Keep the screen on while Grimoire is open;
# Gestures offers no Sleep screen, Sleep now or Sleep after (actions.rs, action_offered), and the keys
# shortcuts.ini binds to them by hand do nothing: the pen keeps writing.
# rust-only
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pen line 300 760 1400 780 60 0.4 0.9
wait 700
expect editor.strokes is 1
# Sleep after is one minute, a second here: it asks nothing, the device's own timer decides.
wait 2500
expect asleep is 0
# Sleep now, Sleep screen and Sleep after, bound to keys by hand: nothing.
key 21 0
wait 200
expect asleep is 0
key 20 0
expect settings.sleep_mode is 2
key 30 0
expect settings.sleep_minutes is 1
pen line 300 900 1400 920 60 0.4 0.9
wait 700
expect editor.strokes is 2
# The system's own sleep: Grimoire shows the page, and wakes on the page.
sleep
wait 400
expect asleep is 1
expect asleep.mode is 0
shot 01-asleep-on-page
wake
wait 400
expect asleep is 0
expect screen is editor
# Sleep settings over the open page: the switch alone (no sleep screen row, no Pin this page, no slider).
# The Clock settings page stays.
pentap panel settings
wait 300
expect editor.modal is settings
tap library settings.cat.2
wait 100
expect library.ui_has.clock-settings is 1
tap library sleep-clock
wait 300
expect editor.settings.sheet is display
expect library.ui_has.display.keep_on is 1
expect library.ui_has.display.sleep is 0
expect library.ui_has.display.pin is 0
expect library.ui_has.display.sleep_after is 0
shot 02-sleep-settings-keep-on-off
tap library display.keep_on
wait 200
expect settings.sleep_minutes is 0
shot 03-sleep-settings-keep-on-on
tap library display.keep_on
wait 200
expect settings.sleep_minutes is 10
tap library display.done
wait 300
expect editor.modal is none
# Gestures: Screen without Sleep screen and Sleep now, System without Sleep after.
tap panel library
wait 200
tap library settings
wait 100
tap library settings.cat.4
wait 100
tap library gestures
wait 100
expect library.sheet is gestures
tap library gestures.group.screen
wait 100
expect library.gestures_group is screen
expect library.ui_has.gestures.row.8 is 1
expect library.ui_has.gestures.row.9 is 0
expect library.gestures.8 has PDF copies
shot 04-gestures-screen
tap library done
wait 100
tap library gestures.group.system
wait 100
expect library.gestures_group is system
expect library.ui_has.gestures.row.5 is 1
expect library.ui_has.gestures.row.6 is 0
expect library.gestures.4 has Language
expect library.gestures.5 has Update channel
shot 05-gestures-system
quit
