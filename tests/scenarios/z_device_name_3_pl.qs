# rust-only
# settings: status_mode=1
# settings: shortcuts_enabled=1
# env: GRIMOIRE_SIM_DEVICE_NAME=Slate GRIMOIRE_LANG=pl GRIMOIRE_CHECK_FIT=1
# copy: tests/fixtures/device-name-keys .
# The same in Polish, which declines the word ("Kindlem", "Kindla"): the device's name takes the place
# of each form. The rotation lock's toast; then the sleep screen's third choice, as the shortcut's toast
# and on the Sleep settings page, whose row is in the shot.
wait 300
key 19 0
wait 100
key 19 0
expect toast is Obrót zgodny z Slate.
wait 100
key 31 0
wait 100
key 31 0
expect settings.sleep_mode is 2
expect toast is Ekran uśpienia: wygaszacz Slate (dotknij, aby zmienić)
tap library settings
wait 300
expect library.sheet is settings
tap library settings.cat.2
wait 100
tap library sleep-clock
wait 300
expect library.sheet is display
shot 01-sleep-settings-on-a-slate
quit
