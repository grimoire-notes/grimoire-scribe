# rust-only
# settings: status_mode=1
# env: GRIMOIRE_SIM_DEVICE_NAME=Slate GRIMOIRE_CHECK_FIT=1
# A platform whose device has a name of its own (sys_device_name): Settings > System > About shows
# that name as its title, where the host and the simulator show "This computer" (z_about.qs).
wait 300
tap library settings
wait 300
expect library.sheet is settings
tap library settings.cat.6
wait 300
tap library about-device
wait 300
expect library.sheet is about-device
expect library.device.name is Slate
shot 01-about-on-a-slate
tap library about-device.back
wait 100
expect library.sheet is settings
quit
