# settings: status_mode=1
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_URL=https://store.test/index.json PATH=tests/store-curl:/usr/bin:/bin GRIMOIRE_BG_DELAY_MS=400
# generate: tests/store_many_packages.py 4 package-store-served
# Package Store (2.3, issue #236): the refreshes of a first visit with a network, nothing saved and no picture
# made yet. A stand-in for curl answers from files of the scenario's own data root, 0.3 s a download
# (tests/store-curl/curl: nothing leaves the machine); the catalog has four packages, and each picture's job is
# held back 0.4 s. A download that ends puts nothing new on the screen (its picture is still to come from the
# sandboxed worker), so it refreshes nothing. From the tap that opens the Store: the tile's press, the page that
# says the Store is being read, the list when the catalog has come, then four ui refreshes, each of an icon's own
# place, 57 x 87 px, and no refresh of the whole screen among them. A package's page: the card's press, the page,
# and one refresh of the screenshot's place when it comes, 721 x 840 px. A screenshot whose download fails (the
# last package's is not served): the page, then only the words that said it was loading, as they go. Before, each
# download's end refreshed the whole screen once more, with nothing new on it. (Each stretch is counted from
# before its tap, so nothing depends on how fast the machine is; the script's clock stands still, so no refresh
# for a new minute falls in.)
clock 1700000000
wait 3600
expect display.presents not -1
tap library app.store
waitfor library.store.list.icons is 4
wait 200
expect library.store.view is list
expect library.store.packages is 4
expect display.presents is fast 842 252 240 200, ui 0 0 1860 2480, ui 0 0 1860 2480, ui 107 379 57 87, ui 701 379 57 87, ui 1295 379 57 87, ui 701 775 57 87
shot 00-four-icons
waitbg
expect display.presents not -1
tap library store.package.com.example.p01
waitfor library.store.shot.loaded is 1
wait 200
expect library.store.view is detail
expect library.store.icon.loaded is 1
expect display.presents is fast 645 338 570 372, ui 0 0 1860 2480, ui 561 1314 721 840
shot 01-a-page
waitbg
tap library store.back
wait 300
expect display.presents not -1
tap library store.package.com.example.p03
waitfor library.store.shot.coming is 0
wait 200
expect library.store.shot.loaded is 0
expect display.presents is fast 645 734 570 372, ui 0 0 1860 2480, ui 724 1786 408 36
shot 02-no-screenshot
tap library store.close
wait 300
expect library.sheet is none
quit
