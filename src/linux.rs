use std::{collections::HashMap, env, ffi::OsString, fs::File, io::{BufRead, BufReader}, path::PathBuf, str::FromStr};

pub const ENV_STEAM_DIR: &str = "STEAM_DIR";

/// Rust is at times writen by bone headed idiots who,
/// just because they are way smarter then all of us still do stuff like this:
/// Refuse a tilde to be resolved on unix systems, but allow a pathbuf with it to be created with
/// no error...
fn expand_tilde(str: &str) -> Option<PathBuf> {
    if let Some(stripped) = str.strip_prefix("~/") {
        if let Some(mut home) = dirs::home_dir() {
            home.push(stripped);
            Some(home)
        } else {
            None
        }
    } else {
        // This was not a home base relative path
        PathBuf::from_str(str).ok()
    }
}

/// This will still return the value, even if `no_tricks` is set
/// Returns the value in env `$STEAM_DIR` (if set)
pub fn get_steam_dir_env_value() -> Option<OsString> {
    env::var_os(ENV_STEAM_DIR)
}

/// This will still return the value, even if `no_tricks` is set
/// Returns the path set in env `$STEAM_DIR`
/// If no path was set returns Err(false)
/// If the value set was not a valid path (or did not exist) returns Err(true)
pub fn get_steam_dir_env_path() -> Result<PathBuf, bool> {
    if let Some(val) = get_steam_dir_env_value() {
        if let Ok(path) = PathBuf::try_from(val) {
            if path.is_dir() {
                return Ok(path);
            }
        }

        Err(true)
    } else {
        Err(false)
    }
}

/// An existing steamroot folder with steamapps and steamruntime
pub struct SteamRoot {
    path: PathBuf,
    steamapps: PathBuf
}

impl SteamRoot {
    pub fn get_root(&self) -> PathBuf {
        self.path.clone()
    }

    /// The steamapps folder in the root directory of steam
    pub fn get_steamapps_folder(&self) -> PathBuf {
        self.steamapps.clone()
    }

    /// Attempts to find the prefix for a given game via it's game id
    pub fn get_prefix(&self, game_id: u32) -> Option<ProtonPrefix> {
        let mut path = self.steamapps.clone();
        path.push("compatdata");
        path.push(game_id.to_string());
        path.push("pfx");

        if let Some(mut pfx) = ProtonPrefix::from_path(path) {
            pfx.game = game_id;
            return Some(pfx);
        }

        None
    }
}

fn has_runtime(steam_root: &PathBuf) -> bool {
    let mut steam_runtime = steam_root.clone();
    steam_runtime.push("ubuntu12_32");

    if steam_runtime.is_dir() {
        return true;
    }

    // Future proofing for 64bit only Steam
    steam_runtime.pop();
    steam_runtime.push("ubuntu12_64");
    
    steam_runtime.is_dir()
}

fn has_steamapps(steam_root: &PathBuf) -> Option<PathBuf> {
    // any spelling of steamapps is apparently valid, so we have to check all folders
    if let Ok(mut iter) = steam_root.read_dir() {
        while let Some(Ok(item)) = iter.next() {
            if item.file_name().to_ascii_lowercase() == OsString::from_str("steamapps").expect("valid os string") && item.path().is_dir() {
                return Some(item.path());
            }
        }
    }

    None
}

/// This verifies that at a given path exists a steam root folder
pub fn steam_root_from(path: PathBuf) -> Option<SteamRoot> {
    if !path.is_dir() {
        return None;
    }
    
    if has_runtime(&path) {
        if let Some(apps) = has_steamapps(&path) {
            return Some(SteamRoot { path, steamapps: apps });
        }
    }

    None
}

/// This will still return the value, even if `no_tricks` is set
/// Returns the steam root from the path set in env `$STEAM_DIR`
/// If no path was set returns Err(false)
/// If the value set was not a valid path (or did not exist) returns Err(true)
pub fn steam_root_env() -> Result<SteamRoot, bool> {
    steam_root_from(get_steam_dir_env_path()?).ok_or(true)
}

// Common Steam Paths
const STEAM_DOT_STEAM: &str = "~/.steam/steam";
const STEAM_LOCAL_SHARE: &str = "~/.local/share/steam";
const STEAM_FLATPAK: &str = "~/.var/app/com.valvesoftware.Steam/data/Steam/";

/// Returns the first steam root found.
/// The Result indicates if an invalid `STEAM_DIR` was set (if you set `no_tricks` you can disgard
/// all Err, it will always return Ok). It return Ok on unset `STEAM_DIR`
/// Err(Some(other)) indicates another standard steam root was found, Err(None) that none.
/// Similarly, Ok(Some(root)) indicates the a root was found (this is the first one), Ok(None) indicates no root was found
///
/// The order in which steam roots are found is:
/// $STEAM_DIR (skipped if `no_tricks`)
/// ~/.steam/steam/
/// ~/.local/share/steam/
/// ~/.var/app/com.valvesoftware.Steam/data/Steam/
pub fn find_steam_root() -> Result<Option<SteamRoot>, Option<SteamRoot>> {
    let err = if cfg!(not(no_tricks)) {
        match steam_root_env() {
            Ok(root) => return Ok(Some(root)),
            Err(err) => err
        }
    } else {
        false
    };

    let path = expand_tilde(STEAM_DOT_STEAM).expect("A Path from home should always resolve");
    if let Some(root) = steam_root_from(path) {
        return match err {
            true => Err(Some(root)),
            false => Ok(Some(root))
        };
    }

    // protontricks checks for both paths, so we will too
    let path = expand_tilde(STEAM_LOCAL_SHARE).expect("A Path from home should always resolve");
    if let Some(root) = steam_root_from(path) {
        return match err {
            true => Err(Some(root)),
            false => Ok(Some(root))
        };
    }

    // Flatpak
    let path = expand_tilde(STEAM_FLATPAK).expect("A Path from home should always resolve");
    if let Some(root) = steam_root_from(path) {
        return match err {
            true => Err(Some(root)),
            false => Ok(Some(root))
        };
    }

    match err {
        true => Err(None),
        false => Ok(None)
    }
}

/// Returns all the steam roots found.
/// The Result indicates if an invalid `STEAM_DIR` was set (if you set `no_tricks` you can disgard
/// all Err, it will always return Ok). It return Ok on unset `STEAM_DIR`
/// If ~/.steam/steam symlinks to ~/.local/share/steam/ then the path will be included only once
///
/// The order in which steam roots are found is:
/// $STEAM_DIR (skipped if `no_tricks`)
/// ~/.steam/steam/
/// ~/.local/share/steam/
/// ~/.var/app/com.valvesoftware.Steam/data/Steam/
pub fn find_all_steam_roots() -> Result<Vec<SteamRoot>, Vec<SteamRoot>> {
    // it will be rare we even get above 2, but still...
    let mut roots = Vec::<SteamRoot>::with_capacity(4);
    let err = if cfg!(not(no_tricks)) {
        match steam_root_env() {
            Ok(root) => {
                roots.push(root);
                false
            },
            Err(err) => err
        }
    } else {
        false
    };

    // Technically, if the user passes in any of the three following paths as the $STEAM_DIR we
    // will have that path twice... not a big deal, but still
    
    let path = expand_tilde(STEAM_DOT_STEAM).expect("A Path from home should always resolve");
    if let Some(root) = steam_root_from(path.clone()) {
        roots.push(root);
    }

    // Usually ~/.steam/steam links to ~/.local/share/steam , so if this is the case we will skip
    // adding what is essentially the same folder twice
    let local_path = expand_tilde(STEAM_LOCAL_SHARE).expect("A Path from home should always resolve");
    let already = if path.is_symlink() {
        if let (Ok(link), Ok(local_path)) = (path.canonicalize(), local_path.canonicalize()) {
            local_path == link
        } else {
            false
        }
    } else {
        false
    };

    if !already {
        if let Some(root) = steam_root_from(local_path) {
            roots.push(root);
        }
    }

    // Flatpak
    let path = expand_tilde(STEAM_FLATPAK).expect("A Path from home should always resolve");
    if let Some(root) = steam_root_from(path) {
        roots.push(root);
    }

    match err {
        true => Err(roots),
        false => Ok(roots)
    }
}

// Name of the important paths
const USER_REG: &str = "user.reg";
const DOS_DEVICES: &str = "dosdevices";
const REG_SHELL_FOLDERS: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Shell Folders";
const REG_VOLATILE: &str = "Volatile Environment";

/// The Proton Prefix for a specfic game, containing the windows like enviroment in which save
/// files and the like are stored
pub struct ProtonPrefix {
    game: u32,
    pfx: PathBuf
}

impl ProtonPrefix {
    /// This can be used to load wineprefixes
    /// Game_id will be set to 0
    pub fn from_path(pfx: PathBuf) -> Option<ProtonPrefix> {
        if pfx.is_dir() {
            let mut dos_devices = pfx.clone();
            dos_devices.push(DOS_DEVICES);

            let mut user_reg = pfx.clone();
            user_reg.push(USER_REG);
            if user_reg.is_file() && dos_devices.is_dir() {
                return Some(ProtonPrefix { game: 0, pfx });
            }
        }

        None
    }

    /// Game ID is 0 for all generic wineprefixes
    pub fn get_game_id(&self) -> u32 {
        self.game
    }

    pub fn get_pfx_path(&self) -> PathBuf {
        self.pfx.clone()
    }

    pub fn get_c_drive(&self) -> PathBuf {
        self.parse_windows_path("C:\\")
    }

    pub fn home(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_VOLATILE, "USERPROFILE")
    }

    pub fn appdata_roaming(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_SHELL_FOLDERS, "AppData")
    }

    fn get_path_from_registry(&self, key: &str, sub_key: &str) -> Option<PathBuf> {
        let mut user_reg = self.pfx.clone();
        user_reg.push(USER_REG);
        if let Some(user_reg) = RegParser::new(user_reg) {
            if let Some(val) = user_reg.open_key(key) {
                if let Some(path) = val.get(sub_key) {
                    return self.parse_windows_path(path).canonicalize().ok();
                }
            }
        }

        None
    }

    /// Turns a string with a absolute windows formated path
    /// into the complete path within this prefix
    pub fn parse_windows_path(&self, str: &str) -> PathBuf {
        if str.is_empty() {
            return self.get_pfx_path();
        }

        let res = str.replace("\\\\", "/"); // paths in reg are written with two \\
        let mut res = res.replace("\\", "/"); // but if someone needs a regualr path converted, this deals with it
        
        let (letter,_) = res.split_at_mut(1);
        letter.make_ascii_lowercase(); // the dirve names in the prefix are lowercase

        let mut path = self.get_pfx_path();
        path.push(DOS_DEVICES);
        path.push(res);

        path
    }
}

/// Returns the first prefix found for this game.
/// There is a chance there are multiple prefixes through multiple steam installs.
/// The Result indicates if an invalid `STEAM_DIR` was set (if you set `no_tricks` you can disgard
/// all Err, it will always return Ok). It returns Ok on unset `STEAM_DIR`
/// Ok(Some(prefix)) indicates the prefix for the game was found, but not necessarily within the
/// set `STEAM_DIR`.
///
/// The search order is:
/// $STEAM_DIR (skipped if `no_tricks`)
/// ~/.steam/steam/
/// ~/.local/share/steam/
/// ~/.var/app/com.valvesoftware.Steam/data/Steam/
pub fn find_prefix(game_id: u32) -> Result<Option<ProtonPrefix>, Option<ProtonPrefix>> {
    let (roots, err) = match find_all_steam_roots() {
        Err(res) => (res, true),
        Ok(res) => (res, false)
    };

    for root in roots {
        if let Some(pref) = root.get_prefix(game_id) {
            return match err {
                true => Err(Some(pref)),
                false => Ok(Some(pref))
            };
        }
    }

    match err {
        true => Err(None),
        false => Ok(None)
    }
}

/// Returns all prefixes found for this game.
/// There is a chance there are multiple prefixes through multiple steam installs.
/// The Result indicates if an invalid `STEAM_DIR` was set (if you set `no_tricks` you can disgard
/// all Err, it will always return Ok). It returns Ok on unset `STEAM_DIR`
///
/// The search order is:
/// $STEAM_DIR (skipped if `no_tricks`)
/// ~/.steam/steam/
/// ~/.local/share/steam/
/// ~/.var/app/com.valvesoftware.Steam/data/Steam/
pub fn find_all_prefixes(game_id: u32) -> Result<Vec<ProtonPrefix>, Vec<ProtonPrefix>> {
    let mut prefixes = Vec::<ProtonPrefix>::with_capacity(4);
    let (roots, err) = match find_all_steam_roots() {
        Err(res) => (res, true),
        Ok(res) => (res, false)
    };

    for root in roots {
        if let Some(pref) = root.get_prefix(game_id) {
            prefixes.push(pref);
        }
    }

    match err {
        true => Err(prefixes),
        false => Ok(prefixes)
    }
}

/// Acts as a wrapper for reading registry entries
pub struct RegParser {
    reg: File
}

impl RegParser {
    /// Creates a wrapper around a .reg Registry file
    pub fn new(reg_file: PathBuf) -> Option<RegParser> {
        if let Ok(file) = File::open(reg_file) {
            Some(RegParser { reg: file })
        } else {
            None
        }
    }
    
    /// Tries to open a given key.
    /// This key has to be formated in a windows path format
    pub fn open_key(&self, key_path: &str) -> Option<HashMap<String, String>> {

        // Serves to read the section header, or determine if there is one at all
        fn read_line_section<'a>(trimed: &'a str) -> Option<&'a str> {
            if let Some(text) = trimed.strip_prefix('[') {
                // Likely found a section header
                if let Some((path,_)) = text.split_once(']') {
                    // split_once insures the character exists, and it doesn't matter if there is a
                    // postfix or not, we are anyway not interested in it

                    return Some(path);
                }
                
            }

            None
        }

        // Serves to read the sub key and write it into the map
        fn read_sub_key(trimed: &str, map: &mut HashMap<String, String>) -> bool {
            if let Some(part) = trimed.strip_prefix('"') {
                if let Some((key, part)) = part.split_once('"') {

                    // This will still not capture dwords, but whatever
                    if let Some(val_part) = part.strip_prefix('=') {
                        if let Some((_,val_untrimmed)) = val_part.split_once('"') {
                            if let Some((val,_)) = val_untrimmed.split_once('"') {
                                map.insert(key.to_string(), val.to_string());
                                return true;
                            }
                        }
                    }

                    // In case we didn't find any other value we insert the valid key
                    map.insert(key.to_string(), String::new());
                    return true;
                }
            }

            false
        }

        // The reg files format paths with \\, in programming \\ becomes one backslash, so \\ is \\\\
        // But since someone could pass a corrected or a standard windows style path, we normalize
        // We do two replace to not accidentally produce  backslashes
        let key_path = key_path.replace("\\\\", "\\").replace("\\", "\\\\");

        let mut reader = BufReader::new(self.reg.try_clone().ok()?);
        let mut output = None;

        let mut line = String::new();
        while let Ok(length) = reader.read_line(&mut line) {
            // read_line does not Err on EOF, it will instead return Ok(0), which we catch
            if length == 0 {
                break;
            }

            let trimed = line.trim();
            
            if let Some(path) = read_line_section(trimed) {
                if output.is_none() && path == key_path {
                    // We found our key
                    output = Some(HashMap::new());
                } else if output.is_some() {
                    break;
                }
                
            }

            if let Some(map) = output.as_mut() {
                read_sub_key(trimed, map);
            }
            
            line.clear();
        }

        output
    }
}
