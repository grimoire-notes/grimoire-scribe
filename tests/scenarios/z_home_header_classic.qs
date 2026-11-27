# root: b_library
# settings: theme=classic
# copy: tests/fixtures/apppkg .
# env: TZ=UTC
# rust-only
# z_home_header.qs in Classic (#332; this was z_home_bat_classic.qs, #319's bat): no bat and nothing on the home
# that opens What's new, Search then Settings at the far right, What's new from the Apps page's foot button, Back
# and the folder's name in a folder, and the row of Files, Package Store and Share, then the installed app, and Show
# All, upright and sideways. The status bar's clock is held still, so the shots are the same on every run.
clock 1700000000
wait 3600
expect screen is library
expect library.sheet is none
waitfor library.apps.icons is 1
expect library.home.header is title search settings
expect library.ui_has.about is 0
expect library.ui_at.114.153 is
expect library.ui_at.400.153 is
expect library.ui_at.114.240 is
expect library.ui_at.1654.153 is search
expect library.ui_at.1762.153 is settings
expect library.ui_at.139.364 is app.files
expect library.ui_at.365.364 is app.store
expect library.ui_at.591.364 is app.share
expect library.ui_at.817.364 is app.com.example.fixture
expect library.ui_at.1721.364 is apps
expect library.ui_has.app.settings is 0
shot 01-home
# Where the bat stood, then the title: neither opens anything.
tap 114 153
wait 100
expect library.sheet is none
tap 400 153
wait 100
expect library.sheet is none
# What's new from the Apps page's foot button, on this version's notes.
tap library apps
wait 100
tap library app.whats-new
wait 100
expect library.sheet is about
expect library.about.view is browser
expect library.about.note is 2.3.0-beta.1
tap library about.close
wait 100
expect library.sheet is none
rotate 1
wait 200
expect library.home.header is title search settings
expect library.ui_has.about is 0
expect library.ui_at.114.153 is
expect library.ui_at.400.153 is
expect library.ui_at.2274.153 is search
expect library.ui_at.2382.153 is settings
expect library.ui_at.140.364 is app.files
expect library.ui_at.360.364 is app.store
expect library.ui_at.580.364 is app.share
expect library.ui_at.800.364 is app.com.example.fixture
expect library.ui_at.2340.364 is apps
shot 02-home-sideways
tap library apps
wait 100
tap library app.whats-new
wait 100
expect library.sheet is about
tap library about.close
wait 100
expect library.sheet is none
rotate 0
wait 200
# In a folder: Back and the folder's name, Search and Settings, nothing that opens What's new.
tap library folder.School
wait 200
expect library.home.header is back title search settings
expect library.ui_at.98.153 is back
expect library.ui_has.about is 0
shot 03-folder
rotate 1
wait 200
expect library.home.header is back title search settings
expect library.ui_at.98.153 is back
expect library.ui_has.about is 0
shot 04-folder-sideways
rotate 0
wait 200
tap library back
wait 200
expect library.home.header is title search settings
expect library.ui_has.about is 0
quit
