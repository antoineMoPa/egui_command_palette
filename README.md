# egui_command_palette

An on-demand command palette for egui applications. It provides search, keyboard navigation, command shortcuts, and a style that can be adjusted through `PaletteStyle`.

```toml
[dependencies]
egui_command_palette = { git = "https://github.com/antoineMoPa/egui_command_palette.git" }
```

Build a list of `Command` values, call `CommandPalette::show` during each UI frame, and handle the returned command ID when the user chooses one.

The crate is licensed under MIT.
