# rust-only
# settings: status_mode=1
# settings: shortcuts_enabled=1
# env: GRIMOIRE_SIM_DEVICE_NAME=Slate GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
# copy: tests/fixtures/device-name-keys .
# The same in Spanish: the translation keeps the word Kindle, and the device's name takes its place.
wait 300
key 19 0
wait 100
key 19 0
expect toast is La rotación sigue al Slate.
# Settings > System > About: its title is the name as it is, in every language.
tap library settings
wait 300
expect library.sheet is settings
tap library settings.cat.6
wait 300
tap library about-device
wait 300
expect library.sheet is about-device
expect library.device.name is Slate
shot 01-about-in-spanish
quit
