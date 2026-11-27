# settings: status_mode=1
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_OFFLINE=1 GRIMOIRE_BG_DELAY_MS=4500
# copy: tests/fixtures/package-updates .
# Packages' pictures (2.3, issue #236): a picture that comes while a finger is down waits for the lift. On the home
# a finger rests on a control of the app row while the installed app's icon comes from the worker (its job is held
# back 4.5 s here): the press's own fast refresh, then nothing while the finger is down, the picture being there
# and not drawn; at the lift, away from the control so that it does nothing, one ui refresh takes the control's
# pressed look away and shows the icon. The same in the Package Store with a finger on a card while its icon
# comes: nothing until the lift, then one ui refresh of the card, the icon in it. (Drawn at once, the page would
# lose the pressed look under the finger.) The script's clock stands still, so no other refresh falls in.
clock 1700000000
wait 3700
expect library.apps is com.example.fixture
expect library.apps.icons is 0
expect display.presents not -1
touch down 1700 350
wait 200
expect display.presents is fast 1607 252 203 200
waitfor library.apps.icons is 1
wait 300
expect display.presents is
shot 00-home-held
touch move 900 1500
touch up 900 1500
wait 300
expect display.presents is ui 924 252 886 200
shot 01-home-lifted
waitbg
tap library app.store
wait 400
expect library.store.view is list
expect library.store.list.icons is 0
expect display.presents not -1
touch down 600 450
wait 200
expect display.presents is fast 348 340 570 372
waitfor library.store.list.icons is 1
wait 300
expect display.presents is
shot 02-store-held
touch move 900 2300
touch up 900 2300
wait 300
expect display.presents is ui 348 340 570 372
expect library.store.view is list
shot 03-store-lifted
quit
