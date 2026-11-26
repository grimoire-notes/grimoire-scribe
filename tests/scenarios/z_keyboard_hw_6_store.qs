# settings: status_mode=1
# settings: shortcuts_enabled=1
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_OFFLINE=1
# copy: tests/fixtures/apppkg .
# copy: tests/fixtures/package-store .
# A paired keyboard types into the Package Store's keyboards and the package manager's (issue #266): the Store's
# search (Enter searches, Esc cancels the keyboard and leaves the Store up) and its repository keyboard, and Install
# from repository... on the All apps page's Manage apps..., each key drawn and refreshed as an on-screen key's text
# is (the text line alone, the keyboard drawn once). Keys by Linux evdev code, the US layout: T 20, I 23, M 50,
# E 18, R 19, X 45, A 30, Backspace 14, Enter 28, Esc 1.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap library app.store
wait 400
expect library.sheet is store
expect library.store.visible is com.example.fixture com.example.timer
tap library store.search
wait 300
key 20 0
key 23 0
key 50 0
key 18 0
key 19 0
key 28 0
wait 300
expect library.sheet is store
expect library.store.query is timer
expect library.store.visible is com.example.timer
# The search again (its keyboard opens with timer): x typed, then Esc cancels the keyboard, not the Store
tap library store.search
wait 300
key 45 0
key 1 0
wait 300
expect library.sheet is store
expect library.store.open is 1
expect library.store.query is timer
# The repository keyboard: a key types (the text line alone), Esc goes back to the list
tap library store.repo
wait 300
expect library.store.view is install-repo
expect display.presents not -1
expect keyboard.draws not -1
key 30 0
expect display.presents is fast 50 1510 1760 120
expect keyboard.draws is 1
key 1 0
wait 300
expect library.store.view is list
expect library.store.open is 1
tap library store.close
wait 300
expect library.sheet is none
# All apps, Manage apps..., Install from repository...: its keyboard takes the keys over the All apps page
tap library apps
wait 200
tap library apps.manage
wait 400
expect editor.packages.open is 1
tap library packages.install
wait 400
expect display.presents not -1
expect keyboard.draws not -1
key 30 0
expect display.presents is fast 50 1510 1760 120
expect keyboard.draws is 1
key 14 0
expect keyboard.draws is 1
key 1 0
wait 300
expect keyboard.draws is 0
expect editor.packages.open is 1
quit
