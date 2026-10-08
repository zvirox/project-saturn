# Easy Edit Pro assets

Place application assets in the matching folder:

- `icons/` — interface icons and toolbar artwork
- `images/` — illustrations, textures, and other raster images
- `animations/` — animation files used by the interface
- `fonts/` — bundled typefaces and their license texts

`icons/saturn-camera.png` is the app logo. It is embedded in the GTK app header and About
dialog, and the desktop window requests the `saturn-camera` icon name.

The bundled Inter Variable font is from the official [Inter project](https://github.com/rsms/inter) and is covered by `fonts/OFL.txt`.

For every third-party asset, record its creator, source, and license in a neighboring `.attribution` file. Keep its license text alongside it when required. Do not add FilmCraft branding or app-logo artwork; Easy Edit Pro uses its own identity.
