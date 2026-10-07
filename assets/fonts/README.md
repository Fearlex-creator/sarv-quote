# Bundled Persian typography

Unmodified Vazirmatn v33.003 static TrueType fonts by the Vazirmatn Project
Authors. Source: https://github.com/rastikerdar/vazirmatn/tree/v33.003/fonts/ttf

The font is licensed under SIL Open Font License 1.1. The complete notice and
license are in `OFL.txt`; upstream attribution is in `AUTHORS.txt`. The font may
be bundled and embedded with software under that license. No proprietary Iran
Bakh, IranYekan, Apple font, or Apple symbol artwork is included.

`src/ui/workspace.rs` embeds these files in the release executable and registers
them only in the current process with Fontconfig. Its guard owns a fresh private
temporary directory and removes it when the application run ends. It does not
install fonts globally, alter user font settings, or require network access.

SHA-256 of the original upstream v33.003 files:

| File | SHA-256 |
| --- | --- |
| Vazirmatn-Regular.ttf | b69fd4c680b8f3f225feabcc655a2c585d97627b8f5f5c0f9985e894069f3a56 |
| Vazirmatn-Medium.ttf | b986623e4ddef10755e04be39f8ea7bcb1dc08bfe8dd0aa6af395736f256ad4a |
| Vazirmatn-SemiBold.ttf | 3f239d8364c14ee32c46b6839fa197c28958358b82a9673d2c6acc8506d78bc5 |
| Vazirmatn-Bold.ttf | f635fdbea28f265de395ba83b4b1570dcf2f58d13c65469e61903b1c2d2ae723 |
