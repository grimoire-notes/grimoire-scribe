# settings: status_mode=1
# rust-only
# While the bar is open and nothing touches the screen it gets the pages its entries go to ready, a band of 160
# rows a tick: a tap then finds both in the cache (a hit, no miss), and nothing in all that, the peek included,
# holds the main loop from reading the pen for long (main.longest_unpumped_ms).
# root: z_bar_0_make
wait 300
tap library card.1
wait 300
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect main.longest_unpumped_ms not -1
# (waited for by what the cache holds, not by a time: a band a pass of the main loop comes as fast as the machine's
# passes do. Two pages held: the first entry's whole, the second's begun; its bands go on in the wait after)
waitfor editor.canvas_cache is 0 0 2
wait 600
tap 479 2218
wait 300
expect editor.canvas_cache is 1 0 2
expect main.longest_unpumped_ms is ok
tap 600 1100
wait 300
expect editor.page is 3
key 48 0
wait 300
expect editor.page is 1
