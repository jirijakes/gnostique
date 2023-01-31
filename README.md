# Gnostique

## About

Native desktop client for Nostr.

## Status

Not yet usable.

Version 0.1.0-alpha.1 does not connect to Nostr, although it already contains basic support for it.
This version is meant for experimentation with constructing widgets and displaying text notes. Events come
from a built-in text file.

## Try now

```
fossil clone https://jirijakes.com/code/gnostique
cd gnostique
cargo run
```

When launched for the first time, avatars will be downloaded and saved to `~/.cache/gnostique/avatars`.
Subsequent launches will use the cached images.

## Screenshot

![](https://jirijakes.com/code/gnostique/doc/tip/doc/history/Screenshot_20230131_115948.png)
