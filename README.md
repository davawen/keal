# Keal

A fast application launcher, that works under wayland, with the convenience and extensibility of ULauncher, but without the cruft or the bugs.  
(or at least, that's my goals ^^')

## Config

TODO: write readme

## Plugins

Plugins are placed in `~/.config/keal/plugins/<name>`.
Characteristics are described in a `config.ini` file:
```ini
[plugin]
prefix = sm ; What the user needs to type
comment = Manage current session ; (optional) Comment shown on the right
exec = exec.sh ; Executable, from the plugin's directory
type = text ; text or json
```

Plugins communicate via `stdio`, the goal is for plugins to be as simple and universal as possible.  

Text plugins are for small, dmenu like utils:
- Keal starts the plugin
- The plugin responds with a list of choices (newline separated)
- Keal sends what choice was selected or wether the user quit
- The plugin takes action

Concretely, here is how communication looks like:
```
(start up)
<- -firefox
<- *com.firefox.icon
<- -chromium
<- =Google's browser
<- -edge
<- %
-> firefox
(launches firefox)
```

Different options are indicated by a symbol followed by their value.
Choices are started with dashes, you can optionally add an icon or a comment with `*` and `=`.
When the choice list is over, send out `%`, and wait on stdin for a response.
