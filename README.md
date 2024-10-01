# Proton-Finder
Rust library for finding and working with steams proton prefixes, and accessing common paths within them.  
Additionally providing a universal wrapper to allow one codebase for Windows and Linux support.  
[Crates.io](https://crates.io/crates/proton-finder)

## Purpose
Primary use case is for developing cross plattform modding tools/save file editors.  
A lot of games save their save files/load mods from the AppData or Documents folders under Windows
(which under Windows are easily accessible with libraries such as [dirs](https://crates.io/crates/dirs)).  
When running under Linux + Proton (or Wine) these folders are stored within the prefix, and therefore harder to access.  
  
So this library provides functions to allow abstracting these prefixes away, 
so you can access the same folders on Windows and Linux through universal functions.  
  
Additional `linux` modul is available when the target_os is Linux, which allows rawer access,  
including opening wine prefixes and reading .reg Registry files from the prefix.  
  

## Example
```
let res = proton-finder::get_game_drive(2420510).map_or_else(|e| {
    println!("Steam Dir provided is not correctly formated, ignored...");
    e
}, |k| k);

if let Some(game_drive) = res {
    if let Some(mut path) = game_drive.config_local_dir() {
        path.push("HoloCure");
        path.push("settings.json");

        if let Ok(text) = std::fs::read_to_string(path) {
            println!("{}", text);
        }
    }
} else {
    println!("Unable to find game drive. Did you install the game?");
}
```
This reads the settings.json for the game HoloCure

## Steam root folder priority
Per default, if the env value `$STEAM_DIR` (same as Protontricks) is set it will use this as the first steam root to search.  
You can disable the automatic reading of this env value with the `no_tricks` features.

After that check the remaining search oder is this:  
`~/.steam/steam`  
`~/.local/share/Steam`  
`~/.var/app/com.valvesoftware.Steam/data/Steam/` (Flatpak)  

## Testing
Requires Steam and [HoloCure](https://store.steampowered.com/app/2420510/HoloCure__Save_the_Fans/)
(free game) installed (and launched the game at least once).
