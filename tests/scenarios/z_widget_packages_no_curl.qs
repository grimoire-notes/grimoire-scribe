# settings: status_mode=1
# rust-only
# env: GRIMOIRE_SIM_DEVICE_NAME=Slate PATH=/nonexistent
# A repository install where no curl can be started (a PATH with no curl on it), on a platform whose device
# has a name of its own (sys_device_name; i18n.rs, the device's name): the error view's text is looked up,
# so it names that device, and curl's exit status follows it as it is. Before, the text was a literal and
# said Kindle on every device.
wait 300
# From the Library: Settings → Packages → Widgets, then Install from repository...
tap library settings
wait 400
tap library settings.cat.5
wait 300
tap library settings.packages
wait 400
expect editor.packages.open is 1
expect editor.packages.error is
tap library packages.install
wait 400
# a/b on the keyboard: a, the symbols page, /, back to the letters, b, then Done installs.
tap 221 2045
wait 100
tap 570 2365
wait 100
tap 1549 2045
wait 100
tap 570 2365
wait 100
tap 1101 2205
wait 100
tap 1685 2365
waitfor editor.packages.error has curl
expect editor.packages.error is The Slate has no curl to fetch packages with. (curl exited 127).
shot 01-no-curl-on-a-slate
quit
