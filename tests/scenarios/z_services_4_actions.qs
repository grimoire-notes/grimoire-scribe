# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/gestures/actions .
# env: GRIMOIRE_SIM_NO_SERVICES=pairing,updates GRIMOIRE_SIM_NO_LIGHT=1
# A device without a front light, on a platform that leaves pairing and updates to its system: the Gestures
# and Buttons lists offer no Light, Warmth, Bluetooth controller or Update channel row (actions.rs,
# action_offered), and the keys and gestures bound to them by hand say there is no front light, or do
# nothing. The rest of their groups is there and acts.
# rust-only
wait 200
key 7 0
expect toast is Front light controls are not available here.
expect light.intensity is 12
key 9 0
expect toast is Front light controls are not available here.
expect light.warmth is 8
key 31 0
expect settings.updates is 0
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
# Bluetooth controller, a three-finger tap here: nothing.
tapn 3 930 1100
wait 100
expect settings.controller is 0
# Sleep screen and Sleep after are still there: this device sleeps when asked.
key 20 0
expect settings.sleep_mode is 1
key 30 0
expect settings.sleep_minutes is 15
tap panel library
wait 200
tap library settings
wait 100
tap library settings.cat.4
wait 100
tap library gestures
wait 100
tap library gestures.group.screen
wait 100
expect library.gestures_group is screen
expect library.gestures.1 has Invert colors
expect library.gestures.2 has Lock rotation
expect library.gestures.8 has Sleep now
expect library.ui_has.gestures.row.9 is 0
shot 01-gestures-screen
tap library done
wait 100
tap library gestures.group.system
wait 100
expect library.gestures_group is system
expect library.gestures.3 has Language
expect library.gestures.4 has Sleep after
expect library.ui_has.gestures.row.5 is 0
shot 02-gestures-system
tap library done
wait 100
tap library settings.back
wait 100
tap library shortcuts
wait 100
tap library buttons
wait 100
tap library buttons.group.system
wait 100
expect library.buttons_group is system
expect library.ui_has.buttons.row.3 is 1
expect library.ui_has.buttons.row.4 is 0
shot 03-buttons-system
quit
