# Iji in Rust!?

Essentially an attempt to implement enough of Game Maker to run Daniel Remar's 2008 game Iji.

To run you will need to unzip the zip file to a `ref` folder - the contents doesn't have any explicit license unfortunately.

You can get the game free at its page: https://www.remar.se/daniel/iji.php - recent versions include the source
Game Maker 7 project (iji.gmk) which is not included here since licensing is a bit squirrelly (there's no explicit
license and the game includes some unlicenced content according to it's manual)

## Progress

- [x] Decodes and can dump the iji.gmk file
- [x] Parses and interprets the custom Game Maker Language (GML) used by Game Maker.
- [x] Just enough bare-bones input and rendering (based on `macroquad`) to get something on screen for testing.
- [x] Enough API implemented to get main menu navigation functional (though it doesn't render correctly).
- [x] A localhost browser-based debugger (essentially just dumps current state for now).
- [ ] Enough API implemented to get New Game working.
- [ ] Sound, music, file-system (saves), etc.
- [ ] Proper GML debugger (breakpoints, etc.)
- [ ] The rest of the owl: Fill out rest of the API, bugs etc., maybe a more "serious" platform than `macroquad`.

There's plenty of interesting ideas from there:
- "build" to an exe (could be just `include_bytes!()`, but I'd like to cook the .gmk file to something friendlier)
- assisted / partially automated translation of the GML code to a "real" language.
- support modding in higher-res resources / fancier rendering techniques

## Licensing

My code implementing the Game Maker side, in particular the GMK format parsing, has referenced existing open source projects in this area:
- https://github.com/IsmAvatar/LateralGM
- https://enigma-dev.org/docs/Wiki/GM_format

While those projects are GPL licensed, no code was substantively copied (e.g. other than referencing constant values
used by the GMK format) other than possibly the Wiki-documented GMK decryption logic, which is required for compatability.
As such I feel safe enough licensing my own code under the more permissive and seemingly preferred by Rust packages' MIT license,
just in case anyone else out there is interested in re-using some of this code porting other GM games (including their own!).

As mentioned, the game content itself is unlicensed, though Daniel Remar is quite clear that he doesn't mind it being
redistributed for free.
