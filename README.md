# Keal

A fast application launcher, that works under wayland, with the convenience and extensibility of ULauncher, but without the occasional slowness, and that is easier to extend.  

## Installation

### With cargo:
```
$ git clone https://github.com/davawen/keal
$ cd keal/keal_iced
$ cargo install --path .
```
`keal` will now be located in `$CARGO_HOME/bin`.  
You can move it to `/usr/local/bin` if you wish to.

## Usage
Simply launch the `keal` executable and search something.
You can use the arrow keys to select, Ctrl+J and Ctrl+K or Ctrl-N and Ctrl-P.

With sway or i3, add this to your config:
```i3config
for_window [title="Keal"] floating enable, border none
```

![Usage gif](/public/readme.gif)

## Features

- [x] Search installed applications and desktop files 
- [x] Configuration (font, style/colors, icon theme)
  - [x] Plugin overrides
  - [x] Plugin configuration
- [ ] Custom aliases
- [x] Frequently launched applications/plugins
- [x] Dmenu mode (with rofi extended protocol)
- [x] Custom plugins 
- [x] Built-in plugins
  - [x] Launch Application
  - [x] List plugins
  - [x] Manage session (log out, suspend, shutdown, ...)
- [ ] Error feedback in UI instead of panicking/logging to stderr
- [ ] Plugin database
- [x] Asynchronous plugin execution

## Configuration
Keal is configured in `~/.config/keal/config.ini`.
```ini
# default values
[keal]
font = Iosevka
font_size = 16.0
font_weight = medium
icon_theme = hicolor
# you can specify multiple icon themes by preference:
#   icon_theme = Zafiro-Icons-Dark,Adwaita,hicolor

# Text shaping may be expensive performance wise, but is necessary if your font does not contain all unicode characters and you need to interact with them.
# Thus it is enabled by default.
# If you know for a fact there won't be any unknown characters, you can set it to `basic`.
text_shaping = advanced

terminal_path = kitty # which terminal to use to launch terminal applications

usage_frequency = true # show the most frequently launched applications first

placeholder_text = search your dreams!

# plugins that you see without typing a prefix
default_plugins = app,ls 

[colors]
# color syntax: `rrggbb` or `rrggbbaa`
background = 24273a

input_placeholder = a5adcb
input_selection = b4d5ff33
input_background = 363a4f

text = cad3f5
matched_text = a6da95
selected_matched_text = eed49f
comment = a5adcb

choice_background = 24273a
selected_choice_background = 494d64 # selected with the keyboard
hovered_choice_background = 363a4f # hovered with the mouse
pressed_choice_background = 181926 # pressed with the mouse

scrollbar_enabled = true # show scrollbar on right side, true or false
scrollbar = 5b6078 # if scrollbar is enabled
hovered_scrollbar = 6e738d
scrollbar_border_radius = 2.0 # floating point number
```

### Plugin configuration

You can override plugin parameters in your `config.ini` like so:
```ini
[keal]
default_plugins = apps,list # remember to update default plugins if you modify prefixes

# starts with the plugin name
[List.plugin]
prefix = list
icon = ./my_list_icon.png # looks in $HOME/.config/keal/
comment = I changed the comment!
```

Additionally, you can edit the config parameters exposed by plugins:
```ini
[Session Manager.config]
log_out = sway exit
suspend = $HOME/run_suspend.sh
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

# Define plugin config options with their default values:
[config]
log_out =
```

Plugins communicate via `stdio`, as to be as simple and universal as possible.  

- At startup:
  - Keal sends the plugin its configuration options (that might have been overriden by the user) in the order they are declared
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
Empty lines are ignored.

- You can subscribe to the following events:
  - `enter`: The user selected or clicked an option. Sends the index of the given choice
  - `shift_enter`: Same, but with shift held
  - `query`: Query string changed. Sends the new query.
- and Keal can take the following actions:
  - `fork`: Closes the window, and continue the plugin as a separate process
      Use this if you wish to launch an application from the plugin
  - `wait_and_close`: Wait for the plugin to end before closing the window
  - `change_input:<value>`: Change's the entire input field (including plugin prefix) to the string following the colon.
      Note that the plugin should terminate after sending this action.
  - `change_query:<value>`: Same as `change_input`, but keeps plugin prefix
  - `update_all`: Replace the current choice list with a new one
  - `update:<index>`: Change a single choice. Give it as a one-element choice list (don't forget the `end`!)
  - `none`: Do nothing

And here is an exemple of a more interactive plugin:
```
<- events:enter shift_enter query
<- name:/
<- name:~
<- end
-> query
-> ~
<- action:update_all
<- name:~/Documents
<- name:~/Pictures
...
<- end
-> enter
-> 1
<- action:update_all
<- name:~/Pictures/Photos
<- name:~/Pictures/image.png
...
<- end
-> shift_enter
-> 0
<- action:fork
(Launches file explorer)
```

## Troubleshooting

### Messed up colors / icons showing as black boxes

Make sure you have graphics drivers installed, `iced` uses `wgpu`, which depends on Vulkan/OpenGL/Metal.

### Hanging when developing a plugin

Make sure you didn't forget to send an `end` after a choice list. Keal will wait indefinitely for it if you forget it!
