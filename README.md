# Proton-Prefix
Rust library for finding and working with steams proton prefixes, and accessing common paths within them.  
Additionally providing a universal wrapper to allow one codebase for Windows and Linux support.  

## Purpose
Primary use case is for developing cross plattform modding tools/save file editors.  
A lot of games save their save files/load mods from the AppData/Documents folders under Windows
(which are easily accessible with libraries such as [dirs](https://crates.io/crates/dirs)).  
When running under Linux + Proton (or Wine) these folders are stored within the prefix.  
  
So this library provides functions to allow abstracting these prefixes away, 
so you can access the same folders on Windows and Linux through universal functions.  
  
Additional linux modul is available when the target_os is Linux to allow also opening wine prefixes,
and reading .reg Registry files from the prefix.

## Example
*TODO*

## Steam root folder priority
Per default, if the env value `$STEAM_DIR` (same as Protontricks) it will use this as the first steam root to search.  
You can disable to automatic reading of the env with the `no_tricks` features.

This is followed by checking:  
`~/.steam/steam`  
`~/.local/share/steam`  
`~/.var/app/com.valvesoftware.Steam/data/Steam/` (Flatpak)  

## Testing
Requires Steam and [HoloCure](https://store.steampowered.com/app/2420510/HoloCure__Save_the_Fans/)
(free game) installed (and launched the game at least once)
