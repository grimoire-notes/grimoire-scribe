# settings: status_mode=1
# What's new is a full-screen browser in the Settings frame: the title and X at the top, every
# version down the left, and the selected version's notes in the reader at the right. Opening it
# selects this version. The Apps page has What's new and About as its two buttons at the foot:
# About opens Settings > System > About, and its Back returns to Settings. The notes are set to fit
# the reader (body_px, lead), upright and sideways. The numbers are this release's notes' again
# (docs/RELEASING.md, The bump commit).
# rust-only
wait 200
expect screen is library
expect library.sheet is none
expect library.ui_has.apps is 1
tap library apps
wait 50
shot 00-apps
# The two foot buttons, left to right.
expect library.ui_at.240.2375 is app.whats-new
expect library.ui_at.644.2375 is app.about
# About links straight to Settings > System > About; Back is Settings.
tap library app.about
wait 100
expect library.sheet is about-device
expect library.device.name is This computer
shot 00-about
tap library about-device.back
wait 100
expect library.sheet is settings
tap library done
wait 100
expect library.sheet is none
# This version's notes in the reader, every version in the sidebar.
tap library apps
tap library app.whats-new
wait 100
expect library.sheet is about
expect library.about.view is browser
expect library.about.full is 1
expect library.about.entries is 6
expect library.about.rows is 6
expect library.about.note is 2.1.0
expect library.about.marked is 2
expect library.about.pages is 1
expect library.about.page is 1
expect library.about.body_px is 43
expect library.about.lead is 1.3
expect library.about.signature is 1
expect library.ui_has.about.close is 1
expect library.ui_has.about.back is 0
shot 01-browser
rotate 1
wait 200
expect library.sheet is about
expect library.about.rows is 6
expect library.about.note is 2.1.0
expect library.about.pages is 1
expect library.about.signature is 1
shot 02-browser-sideways
rotate 0
wait 200
# Another version: the sidebar marks it and the reader shows its notes.
tap library about.row.2.0.0
wait 100
expect library.about.note is 2.0.0
expect library.about.marked is 5
expect library.about.pages is 1
expect library.about.page is 1
expect library.about.body_px is 43
expect library.about.lead is 1.3
expect library.about.signature is 1
expect library.ui_has.about.next is 0
expect library.ui_has.about.prev is 0
shot 03-version
rotate 1
wait 200
expect library.about.note is 2.0.0
expect library.about.pages is 1
expect library.about.signature is 1
shot 04-version-sideways
rotate 0
wait 200
tap library about.close
wait 100
expect library.sheet is none
# Esc closes the browser directly.
tap library apps
tap library app.whats-new
wait 100
expect library.sheet is about
key 1 0
wait 100
expect library.sheet is none
quit
