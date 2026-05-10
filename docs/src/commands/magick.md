# ImageMagick

### `magick`
<p class="cmd-url"><a href="https://imagemagick.org/script/command-line-tools.php">https://imagemagick.org/script/command-line-tools.php</a></p>

- Routing: explicit subcommands match a [[command.sub]] block; bare diagnostic flags match [command.fallback]; otherwise the implicit-convert form delegates to `convert` only when the first positional looks like a file path. The `-script` token is denied anywhere as MSL execution.

- **combine**: Positional args accepted
- **compare**: Positional args accepted
- **composite**: Positional args accepted
- **convert**: delegates to inner command
- **identify**: delegates to inner command
- **mogrify**: Positional args accepted
- **montage**: Positional args accepted
- **stream**: Positional args accepted

- **Fallback grammar (engaged when no sub matches):**
- Allowed standalone flags: --help, --version, -V, -h

