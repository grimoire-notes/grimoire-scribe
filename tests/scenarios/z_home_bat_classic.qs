# root: b_library
# settings: theme=classic
# copy: tests/fixtures/apppkg .
# env: TZ=UTC
# rust-only
# z_home_bat.qs in Classic (#319): the bat left of the home's title at the top level, its tap and the
# title's opening What's new, Back and no bat in a folder, and the row of Files, Package Store and
# Share, then the installed app, upright and sideways. The status bar's clock is held still, so the
# shots are the same on every run.
clock 1700000000
wait 3600
expect screen is library
expect library.sheet is none
waitfor library.apps.icons is 1
expect library.ui_at.114.153 is about
expect library.ui_at.400.153 is about
expect library.ui_at.114.240 is
expect library.ui_at.170.352 is app.files
expect library.ui_at.434.352 is app.store
expect library.ui_at.698.352 is app.share
expect library.ui_at.962.352 is app.com.example.fixture
expect library.ui_has.app.settings is 0
shot 01-home
# The bat, then the title: each opens What's new on this version's notes.
tap 114 153
wait 100
expect library.sheet is about
expect library.about.view is browser
expect library.about.note is 2.3.0-beta.1
tap library about.close
wait 100
expect library.sheet is none
tap 400 153
wait 100
expect library.sheet is about
expect library.about.note is 2.3.0-beta.1
tap library about.close
wait 100
expect library.sheet is none
rotate 1
wait 200
expect library.ui_at.114.153 is about
expect library.ui_at.400.153 is about
expect library.ui_at.170.352 is app.files
expect library.ui_at.434.352 is app.store
expect library.ui_at.698.352 is app.share
expect library.ui_at.962.352 is app.com.example.fixture
shot 02-home-sideways
tap library about
wait 100
expect library.sheet is about
tap library about.close
wait 100
expect library.sheet is none
rotate 0
wait 200
# In a folder: Back where the bat was, the folder's name, nothing that opens What's new.
tap library folder.School
wait 200
expect library.ui_at.114.153 is back
expect library.ui_has.about is 0
shot 03-folder
rotate 1
wait 200
expect library.ui_at.114.153 is back
expect library.ui_has.about is 0
shot 04-folder-sideways
rotate 0
wait 200
tap library back
wait 200
expect library.ui_has.about is 1
quit
