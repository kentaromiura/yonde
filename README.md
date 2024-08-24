## sakubi-reader-app

A `native` [tauri](https://tauri.app/) application implementing colorization from [sakubi reader](https://github.com/kentaromiura/sakubireader),
include an embedded [jitendex](https://jitendex.org/) dictionary that can be enabled with shift,


Changelogs:
===
version 0.2.0
===
since this version the db is exported using https://github.com/kentaromiura/jitendex-analysis,
fixing a potential issue with an unknown license;
db is shrinked from 377 MB to 55M thanks to [zstandard](https://github.com/facebook/zstd);
a better db structure is used as well.

Multiple definitions now works in case same kanji have more than 1 meaning.
Links to other definitions inside a definitions also works now.

MiniYT class now allows dragging
MiniYT allows scrolling
MiniYT definitions has been styled using Jitendex style

TODO:
===
Audio streaming inside definitions and other assets needs to be included...
Initial works was done but need to be rework because safari don't support opus but also seems
in tauri music streaming is also broken atm.
probably will be implemented using [https://github.com/RustAudio/rodio](rodio) on native side.

Screenshots:
===

![main](screenshot/main.png)
![lookup](screenshot/lookup.0.2.png)
![lookup](screenshot/lookup.0.2-1.png)

older:
![lookup](screenshot/lookup.0.1.png)
