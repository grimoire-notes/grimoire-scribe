# settings: status_mode=1
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_URL=https://store.test/index.json PATH=tests/store-curl:/usr/bin:/bin GRIMOIRE_BG_DELAY_MS=400
# generate: tests/store_many_packages.py 6 package-store-served
# Package Store (2.3, issue #236): the refreshes of a first visit with a network, nothing saved and no picture
# made yet. A stand-in for curl answers from files of the scenario's own data root, 0.3 s a download
# (tests/store-curl/curl: nothing leaves the machine); the catalog has six packages, and each picture's job is
# held back 0.4 s. A download that ends puts nothing new on the screen (its picture is still to come from the
# sandboxed worker), so it refreshes nothing: the six icons are six ui refreshes, each of an icon's own place,
# 57 x 87 px, and no refresh of the whole screen; a package's page is one refresh of the screenshot's place when
# it comes, 721 x 840 px. A screenshot whose download fails: only the words that said it was loading are
# refreshed, as they go. Before, each download's end refreshed the whole screen once more, with nothing new on
# it. (The script's clock stands still, so no refresh for a new minute falls among the ones counted.)
clock 1700000000
wait 3600
tap library app.store
waitfor library.store.packages is 6
wait 200
expect library.store.view is list
expect library.store.list.icons is 0
expect display.presents not -1
waitfor library.store.list.icons is 6
wait 200
expect display.presents is ui 107 379 57 87, ui 701 379 57 87, ui 1295 379 57 87, ui 107 775 57 87, ui 701 775 57 87, ui 1295 775 57 87
shot 00-six-icons
waitbg
tap library store.package.com.example.p03
wait 100
expect library.store.view is detail
expect library.store.icon.loaded is 1
expect library.store.shot.coming is 1
expect display.presents not -1
waitfor library.store.shot.loaded is 1
wait 200
expect display.presents is ui 561 1314 721 840
shot 01-a-page
waitbg
# the last package's screenshot is not served: its frame says a screenshot is loading until the download fails
tap library store.back
wait 300
tap library store.package.com.example.p05
wait 100
expect library.store.shot.coming is 1
expect display.presents not -1
waitfor library.store.shot.coming is 0
wait 200
expect library.store.shot.loaded is 0
expect display.presents is ui 724 1786 408 36
shot 02-no-screenshot
tap library store.close
wait 300
expect library.sheet is none
quit
