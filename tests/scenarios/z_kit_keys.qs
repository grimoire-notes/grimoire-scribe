# settings: status_mode=1
# settings: shortcuts_enabled=1
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitinput .
# copy: tests/fixtures/home-keys .
# Keys reach an open app (issue #272), with Input app of tests/fixtures/kitinput (z_kit_input_app.qs says what its
# status line counts: k its KEY events, a its app_key calls) and the shortcuts of tests/fixtures/home-keys (Escape
# back, F files, H home). While an app is open a key calls app_key and kit_event's KEY, pressed and let go, before
# Grimoire's shortcuts and the Library under the app: no shortcut acts but the two that leave the app, back and home.
# App time is the script's (GRIMOIRE_SIM_CLOCK=script); the status bar's clock is held still, in UTC.
clock 1700000000
wait 3600
tap library apps
wait 300
tap library app.test.kit.input
wait 400
expect library.app.open is 1
expect library.app.status is p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
expect display.presents not -1
# A, pressed and let go: app_key twice and two KEY events, then one drawing pass, which changes nothing on the
# screen (the square stays): nothing refreshed.
key 30
keyup 30
wait 300
expect library.app.open is 1
expect library.app.status is p002 n000 t00 r00 s00 m00 e00 c00 q000 k02 a02 g00 h0
expect display.presents is
# F is bound to files: with the app open it is the app's, and Files does not open.
key 33
keyup 33
wait 300
expect library.app.open is 1
expect files.open is 0
expect library.app.status is p003 n000 t00 r00 s00 m00 e00 c00 q000 k04 a04 g00 h0
# H is bound to home: it leaves the app, as the two-finger swipe does, and the app is not told of it.
key 35
keyup 35
wait 300
expect library.app.open is 0
expect screen is library
shot 01-home-left
# Escape is bound to back: it leaves the app too, and goes no further (the Library's top would ask to exit).
tap library apps
wait 300
tap library app.test.kit.input
wait 400
expect library.app.open is 1
key 1
keyup 1
wait 300
expect library.app.open is 0
expect library.sheet is none
# With no app open, the shortcuts are as they were: F opens Files.
key 33
keyup 33
wait 300
expect files.open is 1
quit
