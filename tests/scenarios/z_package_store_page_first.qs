# settings: status_mode=1
# rust-only
# env: GRIMOIRE_PACKAGE_STORE_OFFLINE=1 GRIMOIRE_BG_DELAY_MS=1000
# generate: tests/store_many_packages.py 12
# Package Store (2.3, issue #236): a package's page has its pictures before the list's remaining icons. The saved
# catalog has twelve packages whose pictures the worker has not made yet (each job is held back 1 s here), so the
# list asks for twelve icons, a few jobs at a time. A card is tapped when one icon is there and the second one's
# job runs: the list's jobs still queued leave the queue, the page's screenshot is asked for first, and it is
# made as soon as the running job has ended, with two of the list's icons there and ten still to come. Before,
# the page's screenshot was asked for behind the list's whole queue and came after all twelve. Back at the list,
# its icons go on coming, asked for again as places free up.
clock 1700000000
wait 3600
tap library app.store
wait 400
expect library.store.view is list
expect library.store.packages is 12
waitfor library.store.list.icons is 1
tap library store.package.com.example.p00
wait 100
expect library.store.view is detail
expect library.store.list.icons is 1
expect library.store.icon.loaded is 1
expect library.store.shot.coming is 1
waitfor library.store.shot.loaded is 1
expect library.store.list.icons is 2
shot 00-the-page-first
tap library store.back
wait 300
expect library.store.view is list
waitfor library.store.list.icons is 4
quit
