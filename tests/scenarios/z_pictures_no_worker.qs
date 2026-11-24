# settings: status_mode=1
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_OFFLINE=1 GRIMOIRE_WORK_FAIL_STEP=seccomp
# copy: tests/fixtures/package-updates .
# Packages' pictures with no worker to be had (2.3, issue #236): every worker here refuses a sandbox step before it
# reads a byte, as a Kindle whose sandbox does not hold would. Nothing decodes the pictures then, in a worker or in
# the app: the home's app row keeps the app's built-in icon, a Store card and the detail page keep the package
# glyph, and the screenshot's frame keeps its glyph and does not say a screenshot is loading.
# The first picture's job (the app's icon, asked for by the home) ends having written nothing, and its exit status
# says the sandbox refused its worker a step: from then on no picture is asked for in this run (no job is queued
# for the Store's icon or for the screenshot), and the screens stay usable.
wait 300
expect library.apps is com.example.fixture
waitbg
wait 200
expect library.apps.icons is 0
shot 00-home-built-in-icon
tap library app.store
wait 400
expect library.store.view is list
expect library.store.packages is 2
expect bg.busy is 0
expect library.store.list.icons is 0
shot 01-catalog-glyphs
tap library store.package.com.example.fixture
wait 400
expect library.store.view is detail
expect bg.busy is 0
expect library.store.icon.loaded is 0
expect library.store.shot.loaded is 0
expect library.store.shot.coming is 0
expect library.store.screenshots is 1
shot 02-detail-no-pictures
tap library store.qr
wait 300
expect library.store.view is qr
tap library store.back
wait 300
tap library store.back
wait 300
expect library.store.view is list
tap library store.close
wait 300
expect library.sheet is none
expect library.apps.icons is 0
expect bg.busy is 0
quit
