## Sakubi Reader App

A `native` [tauri](https://tauri.app/) application implementing colorization from [sakubi reader](https://github.com/kentaromiura/sakubireader).

Includes an embedded [jitendex](https://jitendex.org/) dictionary that can be enabled while pressing shift and hovering on a kanji.


Changelogs:
===


version 0.4.0
===

Add a `lookup` menu to make a custom 　query to the dictionary.

Zoom out/in animation for the definition window.


version 0.3.0
===

Using [rodio](https://github.com/RustAudio/rodio) and [magnum](https://github.com/seratonik/magnum-rs) to play sound in dictionary.

Fix other missing assets (svgs).

Implement a global lookup function (only accessible via devTools).



version 0.2.0
===
since this version the db is exported using https://github.com/kentaromiura/jitendex-analysis,
fixing a potential issue with an unknown license.

db is shrinked from 377 MB to 55M thanks to [zstandard](https://github.com/facebook/zstd);
a better db structure is used as well.

Multiple definitions now works in case same kanji have more than 1 meaning.

Links to other definitions inside a definitions also works now.

MiniYT class now allows dragging.

MiniYT allows scrolling.

MiniYT definitions has been styled using Jitendex style.


Screenshots:
===

![main](screenshot/main.png)
![lookup](screenshot/lookup.0.2.png)
![lookup](screenshot/lookup.0.2-1.png)

older:
![lookup](screenshot/lookup.0.1.png)
