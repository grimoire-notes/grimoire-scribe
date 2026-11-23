# settings: status_mode=1
# settings: shortcuts_enabled=1
# What's new's sidebar lists every version's notes (2.1.0, the owner's request of 2026-11-06:
# "wire read all button to a list of each UNIQUE release note, and allow users to easily browse the
# changes by version"): newest first, this version's row selected, each with its version and date.
# A tap on a version shows its notes in the reader, whole on one page (the owner, 2026-11-07:
# "fit all"; about.rs note_fit), with the signature at their end and no page arrows. There is no
# Back layer: the X and Esc close What's new. The rows are docs/whatsnew/'s notes as this build sees
# them (VERSION 2.2.0, with 2.2.0's note written ahead of its release, dated "unreleased" by the
# changelog and not the one marked): every release sets them again (docs/RELEASING.md, The bump
# commit).
# rust-only
wait 200
expect screen is library
tap library apps
tap library app.whats-new
wait 100
expect library.sheet is about
expect library.about.view is browser
expect library.about.full is 1
expect library.about.entries is 6
expect library.about.rows is 6
expect library.about.pages is 1
expect library.about.row.1 is 2.2.0
expect library.about.row_date.1 is unreleased
expect library.about.row.2 is 2.1.0
expect library.about.row_date.2 is 2026-11-13
expect library.about.row.3 is 2.1.0-beta.2
expect library.about.row_date.3 is 2026-11-11
expect library.about.row.4 is 2.1.0-beta.1
expect library.about.row_date.4 is 2026-11-08
expect library.about.row.5 is 2.0.0
expect library.about.row_date.5 is 2026-11-05
expect library.about.row.6 is 2.0.0-beta.1
expect library.about.row_date.6 is 2026-11-01
expect library.about.marked is 2
expect library.about.note is 2.1.0
expect library.ui_has.about.close is 1
expect library.ui_has.about.back is 0
expect library.ui_has.about.next is 0
shot 01-sidebar
# The letter of 2.0.0-beta.1 (it has no title), signed, whole in the reader.
tap library about.row.2.0.0-beta.1
wait 100
expect library.about.note is 2.0.0-beta.1
expect library.about.marked is 6
expect library.about.pages is 1
expect library.about.signature is 1
expect library.ui_has.about.back is 0
expect library.ui_has.about.close is 1
shot 02-letter
# Sideways: the same sidebar and reader, the letter still whole.
rotate 1
wait 200
expect library.sheet is about
expect library.about.view is browser
expect library.about.rows is 6
expect library.about.note is 2.0.0-beta.1
expect library.about.marked is 6
expect library.about.pages is 1
expect library.about.signature is 1
shot 03-letter-sideways
# Another version from the sidebar, still sideways.
tap library about.row.2.1.0
wait 100
expect library.about.note is 2.1.0
expect library.about.marked is 2
expect library.about.pages is 1
expect library.about.signature is 1
shot 04-this-version-sideways
tap library about.close
wait 100
expect library.sheet is none
expect library.about.view is
rotate 0
wait 200
# The keyboard: Esc closes; Right and Left do nothing on notes that fit.
tap library apps
tap library app.whats-new
wait 100
expect library.about.view is browser
key 106 0
wait 50
expect library.about.page is 1
key 105 0
wait 50
expect library.about.page is 1
key 1 0
wait 100
expect library.sheet is none
quit
