# settings: status_mode=1
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_OFFLINE=1
# copy: tests/fixtures/package-store .
# Package Store: the fourth built-in app opens the saved GitHub catalog, filters apps and widgets,
# shows a package's screenshot and icon, offers its repository QR and returns Home. The fixture is
# the real catalog and assets from the content repository, so this checks the offline path.
wait 300
expect library.recent-apps is files,share,settings,store
tap library app.store
wait 400
expect library.sheet is store
expect library.store.open is 1
expect library.store.view is list
expect library.store.packages is 2
expect library.store.visible is com.example.fixture com.example.timer
expect library.store.featured is com.example.fixture
expect library.store.list.icons is 1
shot 00-catalog
tap library store.filter.2
wait 300
expect library.store.filter is 2
expect library.store.visible is com.example.timer
shot 01-widgets
tap library store.package.com.example.timer
wait 500
expect library.store.view is detail
expect library.store.selected is com.example.timer
expect library.store.screenshots is 1
expect library.store.shot.loaded is 1
expect library.store.icon.loaded is 0
shot 02-widget-detail
tap library store.qr
wait 300
expect library.store.view is qr
shot 03-repository
tap library store.back
wait 300
expect library.store.view is detail
tap library store.back
wait 300
expect library.store.view is list
tap library store.filter.0
wait 300
tap library store.package.com.example.fixture
wait 500
expect library.store.view is detail
expect library.store.selected is com.example.fixture
expect library.store.screenshots is 1
expect library.store.shot.loaded is 1
expect library.store.icon.loaded is 1
shot 04-app-detail
# Sideways, the detail page puts its text beside a larger screenshot; the store stays open
# through the re-layout.
rotate 1
wait 400
expect library.store.view is detail
shot 05-app-detail-sideways
rotate 0
wait 300
tap library store.back
wait 300
expect library.store.view is list
tap library store.close
wait 300
expect library.sheet is none
expect library.recent-apps is store,files,share,settings
shot 06-home-store
quit
