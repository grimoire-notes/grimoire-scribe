# settings: status_mode=1
# settings: shortcuts_enabled=0
# rust-only
# env: GRIMOIRE_SIM_CLOCK=script TZ=UTC
# copy: tests/fixtures/kitinput .
# copy: tests/fixtures/home-keys .
# Keys reach an open app with Settings > Shortcuts off too (issue #272), with Input app of tests/fixtures/kitinput
# (k counts its KEY events, a its app_key calls) and the shortcuts of tests/fixtures/home-keys. No shortcut acts then,
# the ones that leave an app neither: Escape, bound to back, is the app's, and the app is left by the two-finger
# swipe as always. (On a Kindle a keyboard is read only while Shortcuts is on.)
# App time is the script's (GRIMOIRE_SIM_CLOCK=script); the status bar's clock is held still, in UTC.
clock 1700000000
wait 3600
tap library apps
wait 300
tap library app.test.kit.input
wait 400
expect library.app.open is 1
expect library.app.status is p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0
key 30
keyup 30
wait 300
expect library.app.status is p002 n000 t00 r00 s00 m00 e00 c00 q000 k02 a02 g00 h0
key 1
keyup 1
wait 300
expect library.app.open is 1
expect library.app.status is p003 n000 t00 r00 s00 m00 e00 c00 q000 k04 a04 g00 h0
key 35
keyup 35
wait 300
expect library.app.open is 1
expect library.app.status is p004 n000 t00 r00 s00 m00 e00 c00 q000 k06 a06 g00 h0
swipe 2 930 2200 0 -1000
wait 300
expect library.app.open is 0
quit
