# Apps

Apps are full-screen programs packaged for Grimoire. They are installed and managed in
**Settings → Packages → Apps**, open from the home app row or the **All apps** page, and run in a
WebAssembly host that gives them the whole display. They are a package kind of their own: an app is
not a widget, and a widget is not an app. Anyone can publish one from a GitHub repository or a
HTTPS `package.yaml`, exactly as widget packages are published.

The package manager, trust prompt, license prompt, settings page, update check and Remove are the
same as Settings → Packages → Widgets. What differs is the folder (`packages/apps` instead of
`packages/widgets`), the index (`packages/apps-index.json`), the manifest type (`type: app`) and
the runtime.

## Package Store

The home app row's fourth built-in app is **Package Store**. It is a catalog of apps and widgets
kept outside Grimoire, in a GitHub repository whose `index.json` names each package's manifest,
artifact, icon, screenshots and description. Opening a package shows its screenshots and metadata,
then Install downloads the manifest and artifact and runs the same trust prompt, digest checks and
installer as **Settings → Packages**. The store does not replace the package manager: installed
packages are still managed there.

The catalog is cached at `grimoire/package-store/index.json`, and its images at
`grimoire/package-store/assets/`, so the last store still opens without a network. The Store itself
never decodes an image: the catalog's icons and screenshots are decoded by the sandboxed picture
worker, as the pictures of Files are ([DESIGN.md](DESIGN.md), "Packages' pictures"), and the
Store draws the gray pictures the worker left in `grimoire/.cache/packages/`. So an icon or a
screenshot shows a moment after its page; until then, and when the worker refuses a picture or
cannot run, a card and the detail page show the package glyph and the screenshot's frame its
glyph. A catalog icon is an `.ico`, a PNG or a JPEG of at most 256 KiB, shown at most 256 px a
side; a screenshot is a PNG or a JPEG of at most 8 MiB, kept at most 2,560 px a side. Grimoire's
catalog is [grimoire-notes/package-store-contents](https://github.com/grimoire-notes/package-store-contents); the
repository's README documents the format, and `tools/check.py` validates a catalog before it is
published. A mirror can be used with `GRIMOIRE_PACKAGE_STORE_URL`.

## A manifest

```yaml
name: Example
id: com.example.app             # lowercase letters, digits, dots and hyphens
version: 1.0.0
type: app
author: Jane Doe
description: A full-screen example.
license: MIT
min_grimoire_version: 2.3.0
platforms: [all]
repository: https://github.com/example/app
artifact:
  path: app.wasm
  sha256: <64 lowercase or uppercase hex digits>
  size: 12345
capabilities: [storage:app-local, network:http]
settings:
  - key: show_clock
    label: Show clock
    type: toggle
    default: true
spec:
  runtime: wasm
  entry: _start                # ignored by the host
  icon: icon.ico               # a built-in icon name, or a bundled .ico
  icon_sha256: <64 hex digits> # required with a .ico icon
  icon_size: 12345             # required with a .ico icon
  status_bar: true             # apps only: start with the shared status bar
```

An omitted `platforms` means every supported device. The artifact is at most 32 MiB. At most 64
app packages are installed. A manifest is at most 256 KiB. The artifact is checked against its size
and SHA-256 before anything is written, and an update is staged before it replaces the old version.

`spec.icon` may name one of Grimoire's built-in icons, or a `.ico` bundled beside `package.yaml`.
A bundled icon is a second package file: the installer fetches it from the manifest's folder, checks
its `spec.icon_sha256` and `spec.icon_size` (at most 256 KiB), and saves it with the artifact. Its
path is never one inside the app's `state/` folder, whatever case the manifest writes that name in
(the storage takes `State/` for `state/`). The
launcher never reads the icon file: the sandboxed picture worker decodes its PNG entries (8 bits a
channel, gray or RGB, with or without alpha; none with a palette or 16 bits) and
classic 1, 4, 8, 24 and 32-bit BMP entries (at most 256 px a side; the largest entry of at most
64 px is taken, else the smallest) to a small grayscale bitmap, kept in
`grimoire/.cache/packages/icons/`, which the home app row, All Apps, the package manager's list
and its detail page draw ([DESIGN.md](DESIGN.md), "Packages' pictures"). A built-in name still
works, and the launcher shows it (or the grid) until the icon's picture is made, and for good if
the icon file is missing, the worker refuses it, or the worker cannot run.

The index is `packages/apps-index.json`:

```json
{
  "version": 1,
  "packages": [
    {"id":"com.example.app","name":"Example","version":"1.0.0","author":"Jane Doe",
     "description":"A full-screen example.","icon":"icon.ico","path":"packages/apps/com.example.app/app.wasm",
     "sha":"…","installed_at":1,"source_url":"https://example.org/app.yaml"}
  ]
}
```

An app's `state/` folder survives an update and a Remove that keeps state. `state/settings.json`
belongs to the host (the values of the manifest's `settings` rows); put the app's own data in
another file under `state/`.

## Capabilities

- `storage:app-local`: the app's own state and settings under its `state/` folder.
- `network:http`: the `http_*` host calls. Without this capability every request is refused.

A package may declare neither, either or both. Unknown capabilities are refused at install.

## Lifecycle and exports

The module exports `memory` and `app_draw`; the rest are optional:

```text
app_draw(width, height, scale_milli, elapsed_ms)
app_pointer(phase, x, y, source)
app_key(code, mods, down)
app_status_bar() -> i32
app_stop()
```

`app_draw` is called whenever Grimoire redraws the app. Width and height are the app body in screen
pixels (the area under the status bar when it is shown), and `scale_milli` is 1000 at the normal UI
scale. `elapsed_ms` is the time since the previous draw, for animation.

`app_pointer` receives every pointer phase in the body: `0` down, `1` move, `2` up. `source` is `0`
for touch and `1` for pen. Coordinates are body-local, so the app never needs to subtract the status
bar itself. A lift outside the body is still delivered so a contact that began in the app can end
cleanly.

`app_key` receives hardware keys. `code` is the Grimoire key code, `mods` its modifier bits, and
`down` is 1 for a press and 0 for a release.

`app_status_bar` is called once, before the first frame. Returning 0 starts with the bar hidden and
1 with it shown; the manifest's `spec.status_bar` is the fallback. The app can change it at any time
with `show_status_bar`.

`app_stop` is called exactly once when the app leaves, whether by the system gesture, its own
`close()`, an update or removal. It is the safe place to flush the app's state. The host saves
`state_set` data after every call anyway, but a last network or file step belongs in `app_stop`.

## The `grimoire` ABI

All strings are UTF-8 in the module's linear memory; a `ptr, len` pair names one. Coordinates and
sizes are screen pixels in the app body.

### Drawing and Grimoire's UI

```text
fill_rect(x, y, w, h, gray)
outline_rect(x, y, w, h, thickness, gray)
round_rect(x, y, w, h, radius, gray, filled, thickness)
line(x0, y0, x1, y1, width, gray)
text(x, y, w, h, ptr, len, px_milli, face, center, gray)
text_at(x, baseline, ptr, len, px_milli, face, gray)
text_width(ptr, len, px_milli, face) -> i32
text_height(px_milli, face) -> i32
icon(name_ptr, name_len, x, y, w, h, gray)
button(x, y, w, h, label_ptr, label_len, icon_ptr, icon_len, style)
chip(x, y, w, h, ptr, len, selected)
row(x, y, w, h, icon_ptr, icon_len, label_ptr, label_len, style, switch)
surface(x, y, w, h, filled, thickness)
select(x, y, w, h, caption_ptr, caption_len, value_ptr, value_len)
app_tile(x, y, w, h, icon_ptr, icon_len, label_ptr, label_len, selected)
fab(x, y, w, h, icon_ptr, icon_len)
card(x, y, w, h, style)
text_field(x, y, w, h, icon_ptr, icon_len, text_ptr, text_len, hint_ptr, hint_len)
clip(x, y, w, h)
clip_clear()
```

Gray is `0` (black) to `255` (paper white). `face` is 0 regular, 1 bold, 2 title, 3 caps, 4 fixed,
5 fixed bold. `style` is 0 plain, 1 selected, 2 disabled, 3 primary. `switch` is `-1` for no switch,
0 off or 1 on. The UI calls draw the same themed elements Settings, the Library and the editor use,
so an app follows the owner's Gothic/Classic theme, fonts and scale.

### State, settings and system

```text
state_get(ptr, cap) -> i32
state_set(ptr, len)
setting(key_ptr, key_len, out_ptr, out_cap) -> i32
status(ptr, len)
request_redraw(after_ms)
close()
show_status_bar(on)
toast(ptr, len, ms)
now() -> i64
now_ms() -> i64
time_text(ptr, cap) -> i32
date_text(ptr, cap) -> i32
battery_percent() -> i32
battery_charging() -> i32
wifi_connected() -> i32
wifi_ssid(ptr, cap) -> i32
screen_width() -> i32
screen_height() -> i32
ui_scale_milli() -> i32
theme() -> i32
status_bar_height() -> i32
```

`state_get`/`state_set` read and write `state/runtime.bin`, capped at 256 KiB. `setting` returns the
saved value for one manifest setting; a missing or invalid value reads as its default. `status`
sets a line a test or a debug view can read. `request_redraw` asks to be drawn again after the
delay (16 ms to 24 hours); timers and polling belong behind it rather than a busy loop. `close`
asks the host to run `app_stop`, save and return Home. `toast` shows a short Grimoire message.

`battery_percent` returns -1 when the device cannot tell. `wifi_connected` returns 1, 0 or -1.
`wifi_ssid` returns the connected network's name, empty when there is none. `time_text` and
`date_text` write human-readable local time and date into the caller's buffer, returning the number
of bytes written. `screen_width`/`screen_height` are the full display, including any status bar;
`status_bar_height` is 0 when no bar is showing.

### HTTP

```text
http_start(method_ptr, method_len, url_ptr, url_len,
           headers_ptr, headers_len, body_ptr, body_len) -> i32
http_poll() -> i32
http_status() -> i32
http_body_len() -> i32
http_body(ptr, cap) -> i32
http_error(ptr, cap) -> i32
http_cancel()
```

`http_start` returns 0 when curl started, -1 for a bad argument or a failed start, -2 when the
manifest lacks `network:http`, and -3 while another request is in flight. The method is 1 to 16
ASCII letters, such as `GET`, `POST`, `PUT`, `PATCH`, `DELETE` or `HEAD`: lower case is read as
upper case, and white space around the method is not part of it (with it, the method's text is at
most 64 bytes). Any other method is a bad argument.

The address begins with `http://` or `https://`, is at most 4096 bytes, has no space and no
control character, and is taken as written: brackets and braces in it are part of the address.
Any other address is a bad argument: `http_start` returns -1 and nothing is sent. A space in an
address is written `%20`. Redirects are followed, at most 5 of them, and only to `http` and
`https` addresses; a request that is redirected further, or elsewhere, ends as a failure. A
request goes to the address it names: a proxy set in Grimoire's environment is not used.

Headers are one `Name: value` per line, at most 32 lines of at most 2048 bytes each (and 32 KiB
together). A header line is a name, a colon and a value: the name is one or more HTTP token
characters (the ASCII letters and digits, and ``! # $ % & ' * + - . ^ _ ` | ~``), the colon
follows the name directly, and the value has no control character. Spaces after the colon are
fine, white space around a line is not part of it, and an empty line is skipped. A request with
any other line, a longer line or more lines is refused whole: `http_start` returns -1 and nothing
is sent. A line with an empty value sends no header: curl then leaves out its own header of that
name, if it has one.

A request without a `User-Agent` line is sent with `User-Agent: Grimoire-App/2.3`. A request
with a `User-Agent` line of its own is sent with that line in its place, never with both. An app
should send its own.

A `HEAD` request has no upload: with one, `http_start` returns -1. It ends when the answer's
header lines have arrived, and those lines are its answer: `http_body` is the status line, the
header lines and the empty line behind them, as the server sent them, of each answer in turn
when redirects were followed. The length those lines state is not counted against the answer's
limit.

The upload is at most 2 MiB and is sent as it is. A request with an upload and no `Content-Type`
line of its own is sent with curl's `Content-Type: application/x-www-form-urlencoded`: an app
that uploads anything else says so with its own line.

The answer is at most 8 MiB (8,388,608 bytes). A longer answer ends
the request: at once when the server states its length, and otherwise once it has grown past the
limit, whether or not the app polls. Such a request has no status code (`http_status` is -1), no
answer, and its `http_error` is `The answer is too large.`

A request has 30 seconds, its redirects included: one that has not ended by then ends as a
failure.

`http_poll` returns 0 while the request runs and 1 when it has finished; after that `http_status`
is the HTTP status code, `http_body` is the answer and `http_error` is empty. A request that curl
ended with a failure (no connection, the time limit, a connection that closed before the whole
answer had arrived) has no status code (`http_status` is -1) and no answer, and its `http_error`
is curl's one-line failure: the part of an answer that had arrived is not given. Poll it after a
`request_redraw`; none of these calls blocks drawing or touching the screen. The host looks after
a request between the app's polls: a request that has ended keeps its outcome for the next
`http_poll`, until the next request starts (`http_start` returns 0) or `http_cancel`. An
`http_start` that returns anything else leaves that outcome as it was.

The host does not provide filesystem or shell access.

## Returning Home

The system gesture is **two fingers swiping up from the bottom of the screen to the middle**. It
works in every app, is not configurable per app, and cannot be disabled by the app. The geometry is
a start in the bottom 28% of the screen, an end no lower than 62% and at least 12% above the start,
with at most 22% horizontal drift.

The app can also call `close()` at any time; both paths run `app_stop`, save state and return to
the Library. Removing the package while it runs does the same before deleting its files.

## Sharing a library

No app-specific code, names or URLs live in Grimoire. The package manager only knows
`type: app`; an app is whatever its module draws. A word game, a GitHub client, a browser or a
calculator with a full-screen mode all use this same ABI. Tests use a fixture app; production code
does not refer to any particular app.
