# rust-only
# generate: tests/previews_fixture.py ghosts
# Pages previews an earlier version drew, current for their page files, as a store updated to 2.3.0 has them: their
# lines are the earlier renderer's (lighter before 2.3.0's darker papers, #314), and their stamps end with the
# generation. Since 2.3.0 a stamp ends with the renderer's fingerprint (store.rs preview_stamp), so none of them is
# current, and the Pages view shows each page as this build draws it, never the earlier picture. Here "ghost"'s page 1:
# its preview is an old picture with ink on it (one pixel in seven dark) while the page is cleared (no ink at all),
# so the tile drawn again has only the one pixel of Narrow rule's header crossing its margin darker than mid-gray.
# (Previews this build drew stay current from one start to the next: tests/pages_cache_check.py, whose second process
# finds every preview the first one made.)
wait 300
tap library notebook.ghost
expect editor.page is 2
tap panel pages
expect editor.modal is pages
waitbg 30000
wait 300
expect editor.pages.ink.1 is 1
shot 01-tile-drawn-again
quit
