# rust-only
# settings: shortcuts_enabled=1
# env: GRIMOIRE_WIFI_SCRIPT=tests/fixtures/wifi/helper.sh
# A paired keyboard types a Wi-Fi password (issue #266): on the masked keyboard, as its keys do, drawn as stars and
# never in a probe (library.caret stays empty); Backspace deletes, Esc cancels, Enter is Done. Seven characters are
# refused (a WPA password has 8 to 63), eight are added. Other...'s hidden network name is typed the same way.
# Keys by Linux evdev code, the US layout (mods 2 is Shift): H 35, U 22, N 49, T 20, E 18, R 19, C 46, A 30, B 48,
# I 23, the digits 2 to 11, Backspace 14, Enter 28, Esc 1.
wait 3600
tap library settings
wait 100
tap library wifi
wait 300
expect library.sheet is wifi
tap library wifi.radio
wait 800
expect wifi.state is connected
expect wifi.saved is 15
expect wifi.near is 5
# Neighbor (WPA3, not saved): its masked password keyboard. Esc cancels it, nothing added.
tap library wifi.near.4
wait 100
expect library.sheet is keyboard
key 30 0
key 48 0
expect library.caret is
key 1 0
wait 300
expect library.sheet is wifi
expect toast is
expect wifi.saved is 15
# hunter2: seven characters, which a WPA password cannot be
tap library wifi.near.4
wait 100
expect library.sheet is keyboard
key 35 0
key 22 0
key 49 0
key 20 0
key 18 0
key 19 0
key 3 0
expect library.caret is
shot 01-masked
key 28 0
wait 800
expect toast is Could not add the network.
expect library.sheet is wifi
expect wifi.saved is 15
# Hunter22! with a Backspace: Hunter22, eight characters, added
tap library wifi.near.4
wait 100
key 35 2
key 22 0
key 49 0
key 20 0
key 18 0
key 19 0
key 3 0
key 3 0
key 2 2
key 14 0
expect library.caret is
key 28 0
wait 800
expect wifi.saved is 16
expect wifi.current is Neighbor
# Other...: the hidden network's name, typed and shown (no password), then its security and its password
tap library wifi.other
wait 100
expect library.sheet is keyboard
key 46 2
key 30 0
key 48 0
key 23 0
key 49 0
expect library.caret is Cabin|
key 28 0
wait 300
expect library.sheet is wifi
tap library wifi.other.wpa
wait 100
expect library.sheet is keyboard
key 2 0
key 3 0
key 4 0
key 5 0
key 6 0
key 7 0
key 8 0
key 9 0
expect library.caret is
key 28 0
wait 800
expect wifi.saved is 17
expect wifi.current is Cabin
quit
