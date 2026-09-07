# Fire Notes visual design

Fire Notes is a compact writing tool: a black canvas, warm monospace text, a thin
tab strip, and fire responding to writing and selection. The editor gets the space
and attention. Controls should look like they belong to that same tool.

## Review and direction

The command palette had blue outlines and selection fills, a second outlined
search field, gray text, and excess space below its rows. The note picker shared
those choices, while the context menu, tabs, selection, and status text used
different palettes. Each was readable alone; together they lacked a common style.

Use one warm palette across all app-owned controls. Popups have one quiet outer
border, a flat search field, and a divider separating input from results. Keep
spacing balanced and give actions the same names wherever they appear. Reserve
bright orange for active tabs, the caret, open-note indicators, and controls.

## Color

The code source is [design.rs](apps/fire-notes/src/design.rs). Apply these tokens
through the framework's `Theme`; Fire Notes' palette is not a framework default.

| Role | Color | Use |
| --- | --- | --- |
| Canvas | `#000000` | Writing background |
| Chrome | `#100909` | Tab strip and resting window controls |
| Panel | `#201613` | Menus, pickers, and their search fields |
| Text | `#FFE6CC` | Note text, active tab, popup labels |
| Muted | `#B69B86` | Inactive tabs, shortcuts, placeholders, status |
| Ember | `#FF8A36` | Interaction accents |

Supporting colors stay in the same family: `#3B261D` for hovered or selected
actions, `#62432F` for popup borders, `#291510` for active tabs, and `#422619`
for text selection. A hovered window-close button uses `#792C21` with light text.
Fire itself retains its orange, red, and yellow animation colors.

## Type and spacing

- Use the existing monospace font. Note text is 16 logical pixels; tabs, search,
  actions, and shortcut hints are 14. Status text is 12.
- Keep the tab strip 40 pixels high and editor padding at 16 horizontal, 8 vertical.
- Popup rows are 32 pixels high. Text and shortcut hints share a vertical center.
  Shortcuts align right; titles clip before hints and open-note indicators.
- The command palette is at most 360 pixels wide; note and file pickers are at
  most 420. Leave at least 8 pixels between a popup and the window edge.
- Picker content has 8-pixel outer padding, a 36-pixel search field, and a small
  separation before results. Match padding above and below the content.
- Popups have a 6-pixel radius and a 1-pixel border. Selected rows use a 3-pixel
  radius. Keep the editor and tab strip square.

## Interaction

The active tab has a 2-pixel orange top edge and brighter text. Hover backgrounds
are restrained. Window controls gain a visible outline for keyboard focus, and
the close control changes color on hover.

Menus and pickers share colors, text sizing, and action names. Show shortcuts next
to commands. An orange dot identifies an open note; a check identifies an enabled
toggle in the tab menu. Disabled actions remain legible. Empty searches explain
that there are no matches and retain the editable query.

The command palette stays near the caret without covering its line when space
allows. Note lookup is centered. Popups stay inside the window at minimum size.
Escape and outside clicks dismiss them and return focus to writing.

Trash uses the same searchable popup, with remaining recovery time beside each
note and a footer stating the 30-day retention policy. Selecting a note restores
it. “Move to Trash” is separate from “Close tab.” Trashing the last tab leaves
keyboard hints for creating, finding, and restoring notes.

## Motion and verification

Fire is the distinctive motion. Keep its response to typing and selected text;
let it settle when writing stops. Menus, hover, and focus do not start animations
or recurring timers. Maintain responsive resizing and low idle CPU use.

Check native screenshots of writing, selected fire, tabs and rename, the command
palette, note lookup, empty search, context actions, and narrow windows. Run
`tools/notes_probe.py` and `tools/tab_menu_probe.py` against a release build using
temporary notes. Verify keyboard and pointer behavior as well as appearance.
