# rust-only
# settings: status_mode=1
# env: GRIMOIRE_CHECK_FIT=1
# Settings > System > About on the simulator, which says Kindle for the device's name
# (sys_device_name): the title is "This computer", the line for a computer. A platform with a name of
# its own shows that name there (z_device_name_3_about.qs).
wait 300
tap library settings
wait 300
expect library.sheet is settings
tap library settings.cat.6
wait 300
tap library about-device
wait 300
expect library.sheet is about-device
expect library.device.name is This computer
shot 01-about-on-the-simulator
tap library about-device.back
wait 100
expect library.sheet is settings
quit
