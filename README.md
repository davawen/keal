# Keal

A fast application launcher, that works under wayland, with the convenience and extensibility of ULauncher, but without the cruft or the bugs.  
(or at least, that's my goals ^^')

## Features

- [x] Search installed applications and desktop files 
- [x] Configuration (font, style/colors, icon theme)
  - [ ] Plugin overrides
- [ ] Custom aliases
- [ ] Frequently launched applications/plugins
- [ ] Dmenu mode (with rofi extended protocol)
- [x] Custom plugins 
- [ ] Built-in plugins (session, list installed plugins, ...) 
- [ ] Plugin error feedback instead of panicking
- [ ] Plugin database

## Configuration
Keal is configured in `~/.config/keal/config.ini`.
```ini
# default values
[keal]
font = Iosevka
font_size = 16.0
font_weight = medium
icon_theme = hicolor
```

## Plugins

Plugins are placed in `~/.config/keal/plugins/`.
Characteristics are described in a `config.ini` file:
```ini
[plugin]
name = Session Manager
icon = user # (optional) Plugin icon
  # An icon can be the name of one in the icon theme, an absolute path, or a relative path (by starting with "./")
  # Note that this works for plugin icons and for choice icons
comment = Manage current session # (optional) Comment shown on the right
prefix = sm # What the user needs to type
exec = exec.sh # Executable, from the plugin's directory
```

Plugins communicate via `stdio`, as to be as simple and universal as possible.  

- At startup:
  - The plugin tells which events it wants to be subscribed to
  - The plugin responds with an initial choice list (newline separated)
- Then, in a loop:
  - Keal sends an event to the plugin
  - The plugin responds with an action keal should take
This repeats until either the plugin asks keal to close, or the user quits the plugin

Concretely, here is how communication looks like:
```
(start up)
<- events:enter
<- name:firefox
<- icon:com.firefox.icon
<- name:chromium
<- comment:Google's browser
<- name:edge
<- end
-> enter
-> 0
<- action:fork
(launches firefox)
```

Different options are indicated by a field name, a colon, and a value.
A choice list expects `name:`s, with optional icons and comments, finished with an `end`.

- Keal can take the following actions:
  - `fork`: Closes the window, and continue the plugin as a separate process
      Use this if you wish to launch an application from the plugin
  - `wait_and_close`: Wait for the plugin to end before closing the window
  - `change_input:<value>`: Change's the entire input field (including plugin prefix) to the string following the colon.
      Note that the plugin should terminate after sending this action.
  - `change_query:<value>`: Same as `change_input`, but keeps plugin prefix
  - `update-all`: Replace the current choice list with a new one
  - `update:<index>`: Change a single choice. Give it as a one-element choice list (don't forget the `end`!)
  - `none`: Do nothing
- And you can subscribe to the following events:
  - `enter`: The user selected or clicked an option. Sends the index of the given choice
  - `shift_enter`: Same, but with shift held
  - `query`: Query string changed. Sends the new query.

And here is an exemple of a more interactive plugin:
```
<- events:enter shift_enter query
<- name:/
<- name:~
<- end
-> query
-> ~
<- action:update-all
<- name:~/Documents
<- name:~/Pictures
<- end
-> enter
-> 1
<- action:update-all
<- name:~/Pictures/Photos
<- end
-> shift_enter
-> 0
<- action:fork
(Launches file explorer)
```
