# settings: status_mode=1
# settings: theme=classic
# What's new in Classic: 2.0.0's notes in the full-screen reader. Upright they fit at 44 px in the
# look they always had. Sideways the narrower reader tightens the leading to 1.23, still on one
# page. The counts are the notes' (body_px, lead): every release's new notes set them again
# (docs/RELEASING.md, The bump commit).
# rust-only
wait 200
expect screen is library
tap library apps
tap library app.whats-new
wait 100
expect library.sheet is about
expect library.about.view is browser
tap library about.row.2.0.0
wait 100
expect library.about.note is 2.0.0
expect library.about.pages is 1
expect library.about.body_px is 44
expect library.about.lead is 1.3
expect library.about.signature is 1
expect library.ui_has.about.back is 0
shot 01-note
rotate 1
wait 200
expect library.sheet is about
expect library.about.note is 2.0.0
expect library.about.pages is 1
expect library.about.body_px is 44
expect library.about.lead is 1.23
expect library.about.signature is 1
expect library.ui_has.about.next is 0
shot 02-note-sideways
tap library about.close
wait 100
expect library.sheet is none
quit
