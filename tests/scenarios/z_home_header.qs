# root: b_library
# settings: status_size=1
# copy: tests/fixtures/apppkg .
# env: TZ=UTC
# rust-only
# The home's header (#332; this was z_home_bat.qs, #319's bat): the title at the left with nothing beside it, and
# nothing on it that opens What's new (a tap on the title or where the bat stood opens nothing); Search, then
# Settings at the far right. What's new opens from the Apps page's foot button, on this version's notes. In a
# folder Back and the folder's name are there. Under the header the row is Files, Package Store and Share, then the
# installed app, and Show All in the row's last square. Gothic, upright and sideways (z_home_header_classic.qs: the
# same in Classic, at the small status bar; this one at the medium bar, z_home_ruled at the small and the large). The
# status bar's clock is held still, so the shots are the same on every run.
clock 1700000000
wait 3600
expect screen is library
expect library.sheet is none
waitfor library.apps.icons is 1
expect settings.status_size is 1
# (the medium bar: everything under the header 6 px lower than at the small one, the foot where it was)
expect library.home.squares is 51 276 176 176, 277 276 176 176, 503 276 176 176, 729 276 176 176, 1633 276 176 176
expect library.home.marks is title 233 11, break 540 32, light 709 2, foot 2235 11
expect library.home.marks_in_hits is 0
expect library.home.header is title search settings
expect library.ui_has.app.whats-new is 0
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
expect library.ui_has.app.whats-new is 0
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
expect library.ui_has.app.whats-new is 0
shot 03-folder
rotate 1
wait 200
expect library.home.header is back title search settings
expect library.ui_at.98.153 is back
expect library.ui_has.app.whats-new is 0
shot 04-folder-sideways
rotate 0
wait 200
tap library back
wait 200
expect library.home.header is title search settings
expect library.ui_has.app.whats-new is 0
quit
