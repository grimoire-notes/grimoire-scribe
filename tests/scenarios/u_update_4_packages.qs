# settings: status_mode=1
# settings: updates=preview
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_OFFLINE=1 GRIMOIRE_UPDATE_SCRIPT=tests/fake_update.sh FAKE_UPDATE_PREVIEW=9.0.0-beta.9 FAKE_UPDATE_STABLE=1.2.0
# copy: tests/fixtures/package-updates .
# Settings > Updates is the updater for Grimoire and for installed packages: the system card carries
# the branch's newer version, and the Package Updates list shows each installed package's icon, name
# and version change (the app's cached Store icon, the widget's own glyph). The fixture holds an
# outdated app and widget, and a newer catalog, all offline.
wait 300
tap library settings
wait 300
tap library settings.cat.0
wait 200
tap library updates
wait 900
expect library.sheet is updates
expect library.update is newer 9.0.0-beta.9
expect library.update.packages is com.example.fixture com.example.timer
expect library.update.pkg.versions is com.example.fixture=1.0.0->1.2.0 com.example.timer=1.0.0->1.1.0
expect library.update.pkg.status is idle
expect library.update.pkg.icons is 1
expect library.ui_has.updates.pkg.0 is 1
expect library.ui_has.updates.pkg.1 is 1
shot 00-updates
rotate 1
wait 400
expect library.update.packages is com.example.fixture com.example.timer
shot 01-updates-landscape
rotate 0
wait 300
tap library updates.channel
wait 100
expect settings.updates is 1
wait 900
expect library.update is newest 1.2.0
shot 02-stable
tap library settings.back
wait 300
expect library.sheet is settings
quit
