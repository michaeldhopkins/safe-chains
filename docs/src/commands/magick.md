# ImageMagick

### `magick`
<p class="cmd-url"><a href="https://imagemagick.org/script/command-line-tools.php">https://imagemagick.org/script/command-line-tools.php</a></p>

- Bare flags: --help, --version, -V, -h
- Subcommand-explicit form: `magick <sub> <args>` for sub in {compare, composite, convert, identify, mogrify, montage, stream, combine}
- Convert-implicit form: `magick <input> [options] <output>` — equivalent to `magick convert <input> [options] <output>`, validated by the convert top-level command surface
- `magick identify ...` is Inert (read-only); other forms are SafeWrite

