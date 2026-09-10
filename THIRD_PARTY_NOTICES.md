# Third-party notices

AlertView itself is under the [MIT License](LICENSE). It also ships artwork it
did not draw, listed here with its own terms.

## Noto Color Emoji

Three emoji drawings are embedded in `static/style.css` as `data:` URIs and
served as part of the dashboard:

| Icon name | Emoji | Source file |
|---|---|---|
| `flame` | 🔥 U+1F525 | `svg/emoji_u1f525.svg` |
| `bell-off` | 🔕 U+1F515 | `svg/emoji_u1f515.svg` |
| `hourglass` | ⏳ U+23F3 | `svg/emoji_u23f3.svg` |

**Origin:** [googlefonts/noto-emoji](https://github.com/googlefonts/noto-emoji),
commit `8998f5dd683424a73e2314a8c1f1e359c19e8742`.

**Copyright:** © 2013 Google LLC.

**Licence:** SIL Open Font License, Version 1.1 — the full text is in
[LICENSES/OFL-1.1-Noto.txt](LICENSES/OFL-1.1-Noto.txt), copied verbatim from
that repository's `LICENSE` file.

> **A note on which licence applies.** The upstream repository is not
> self-consistent: its README states that "Tools and most image resources are
> under the Apache license, version 2.0" and links that sentence to `./LICENSE`
> — a file which actually contains the SIL Open Font License 1.1, and whose
> first line reads "This Font Software is licensed under the SIL Open Font
> License, Version 1.1". Rather than pick the reading that suits us, AlertView
> complies with the licence file that is actually there. Complying with the OFL
> also satisfies the Apache-2.0 reading, since both require no more than
> retaining the notice, which this file does.

**Modifications:** the three SVG files were minified for embedding — the XML
prolog, the generator comment, the decorative root `id`, and the inert
`enable-background`, `version`, `x`, `y` and `xml:space` attributes were
removed, whitespace was collapsed, and the result was percent-encoded into a
CSS `data:` URI. **The drawings are unmodified**: the element tree and every
geometry, colour and gradient attribute are identical to the originals, which
was verified by comparing the parsed trees before and after.

No Reserved Font Name is used, and none of this software is sold by itself.

## Everything else

The rest of AlertView — the Rust backend, the frontend, the remaining icons
(the app logo, the toolbar glyphs, the ↗ link marker) — is original work under
the MIT License.
