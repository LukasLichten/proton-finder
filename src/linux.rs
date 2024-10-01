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

/// Returns the value in env `$STEAM_DIR` (if set)
///
/// This will still return the value, even if `no_tricks` is set
pub fn get_steam_dir_env_value() -> Option<OsString> {
    env::var_os(ENV_STEAM_DIR)
}

/// Returns the path set in env `$STEAM_DIR`
///
/// This will still return the value, even if `no_tricks` is set
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
#[derive(Debug, Clone)]
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

    /// Returns the library in which the game is installed.
    /// The game prefix data might be elsewhere (like with the Steamdeck SD-card libraries store
    /// compatdata still in the steamroot).
    ///
    /// This is based on the libraryfolder.vdf file, which steam updates infrequently, meaning if
    /// you just moved the game it will still be noted with it's old location (so this value is
    /// unfortunatly not that reliable)
    pub fn get_install_library(&self, game_id: u32) -> Option<SteamLibrary> {
        let vdf = self.read_library_folders_vdf_file()?;
        let game_id = game_id.to_string();

        for (_,lib) in vdf.pairs.iter() {
            if let VdfValue::Complex(lib) = lib {
                
                // Extracting necessary values
                if let (Some(VdfValue::Simple(path)),Some(VdfValue::Complex(apps))) = (lib.pairs.get("path"), lib.pairs.get("apps")) {
                    
                    if apps.pairs.contains_key(&game_id) {
                        // Found the game
                        
                        let buf = PathBuf::from_str(path).ok()?;
                        return SteamLibrary::from_path(&buf);
                    }
                }
            }
        }

        None
    }

    /// Attempts to find the prefix for a given game via it's game id
    pub fn get_prefix(&self, game_id: u32) -> Option<ProtonPrefix> {
        if let Some(lib) = self.get_install_library(game_id) {
            if let Some(pre) = lib.get_prefix(game_id) {
                // Found prefix already
                return Some(pre);
            }
        }

        // As a fallback we iterate through all libraries
        // Especially duye to the vdf being potentially out of date
        for lib in self.get_libraries() {
            if let Some(pre) = lib.get_prefix(game_id) {
                // Found prefix already
                return Some(pre);
            }
        }

        // Root is "garanteed" to be included, so no need to recheck
        None
    }

    /// Returns you all libraries part of this steamroot
    /// This function always returns at least 1 result, that being the root library
    pub fn get_libraries(&self) -> Vec<SteamLibrary> {
        let mut res = Vec::<SteamLibrary>::new();

        if let Some(vdf) = self.read_library_folders_vdf_file() {
            // Iterating over all entires
            for (_,lib) in vdf.pairs.iter() {
                if let VdfValue::Complex(lib) = lib {

                    // retrieving the path for this library
                    if let Some(VdfValue::Simple(p)) = lib.pairs.get("path") {
                        
                        // Parsing into wrapper
                        if let Ok(path) = PathBuf::from_str(p) {
                            if let Some(item) = SteamLibrary::from_path(&path) {
                                res.push(item);
                            }
                        }
                    }
                }
            }
        }


        if res.is_empty() {
            // Fallback to garantee at least the root exists
            res.push(SteamLibrary { steamapps: self.get_steamapps_folder(), is_root: true });
        }

        res
    }

    /// Reads the libraryfolders file for this streamroot,
    /// returning on success the contained "libraryfolders" struct (so you can directly access the
    /// libraries).  
    ///
    /// This is in contrast to calling `parse_vdf_file` manually, which would give you the root
    /// object that contains this struct under said key (so this function here saves you one step).
    pub fn read_library_folders_vdf_file(&self) -> Option<VdfStruct> {
        let mut path = self.get_steamapps_folder();
        path.push("libraryfolders.vdf");

        // We use remove here to avoid a clone call
        let mut vdf = parse_vdf_file(&path)?;
        if let Some(VdfValue::Complex(res)) = vdf.pairs.remove("libraryfolders") {
            Some(res)
        } else {
            None
        }
    }
}

/// Parses a vdf file at the given location
///
/// This was made to parse the libraryfolders.vdf,
/// so other vdf files might not get properly parsed
pub fn parse_vdf_file(file_path: &PathBuf) -> Option<VdfStruct> {

    // Parses structs recusrively
    fn parse_struct(reader: &mut BufReader<File>, root: bool) -> Option<VdfStruct> {
        let mut obj = VdfStruct { pairs: HashMap::new() };
        
        let mut line = String::new();
        while let Ok(length) = reader.read_line(&mut line) {
            // EOF detection
            if length == 0 {
                if root {
                    return Some(obj);
                } else {
                    return None;
                }
            }

            let trimed = line.trim();
            
            if trimed == "}" {
                // Object ended
                return Some(obj);
            }

            // Key parsing
            if let Some(part) = trimed.strip_prefix('"') {
                let (key, part) = part.split_once('"')?;

                let part_trimed = part.trim();

                let (key, value) = if let Some((_,val_untrimmed)) = part_trimed.split_once('"') {
                    // This handles simpletype
                    let (val,_) = val_untrimmed.split_once('"')?;
                    (key.to_string(), VdfValue::Simple(val.to_string()))
                } else {
                    // This handles complextype by reading another line to find the bracket
                    // open, and then do recursion
                    let key = key.to_string();

                    line.clear();
                    if reader.read_line(&mut line).ok()? == 0 {
                        // EOF between the key and the struct
                        return None;
                    }

                    let trimed = line.trim();
                    if trimed != "{" {
                        // Unexpected symbol
                        return None;
                    }

                    let s = parse_struct(reader, false)?;
                    (key, VdfValue::Complex(s))
                };

                obj.pairs.insert(key, value);
            } else if !trimed.is_empty() {
                return None;
            };

            line.clear();
        }

        None
    }


    let file = File::open(file_path).ok()?;
    let mut reader = BufReader::new(file);

    parse_struct(&mut reader, true)
}

/// Represents a Vdf Complextype with multiple key value pairs, where the value can be further nested structs
#[derive(Debug, Clone)]
pub struct VdfStruct {
    pub pairs: HashMap<String, VdfValue>
}

/// Represents the two value types for vdf:
/// - Simpletype, which is a String value on the same line as it's key
/// - Complextype, which is a struct started with { and ended with } on seperate lines
#[derive(Debug, Clone)]
pub enum VdfValue {
    Complex(VdfStruct),
    Simple(String)
}


/// Wrapper around a SteamLibrary with a compatdata folder
#[derive(Debug, Clone)]
pub struct SteamLibrary {
    steamapps: PathBuf,
    is_root: bool
}


impl SteamLibrary {
    /// Produces a new wrapper for the given location, as long as a compatdata folder is present
    ///
    /// Important: You are passing in the library folder, as set in steam, not the contained
    /// steamapps folder!
    pub fn from_path(lib: &PathBuf) -> Option<Self> {
        let mut apps = has_steamapps(lib)?;
        apps.push("compatdata");
        if !apps.exists() {
            return None;
        }
        apps.pop();
        
        Some(Self { steamapps: apps, is_root: has_runtime(lib) })
    }

    /// Attempts to find the prefix for a given game via it's game id.  
    ///
    /// This only checks if there is a prefix for the game in THIS library, so:  
    /// - The game might be installed here, but the prefix is left in the root (Steamdeck SD-Card behavior)
    /// - There is leftover data from the game being here that has not been cleaned up (you get a
    /// prefix then, but you shouldn't use it, as it is irrelevant to the current install of the game)
    /// - The game is in another library (then you need to check the other Libaries).
    /// 
    /// In general, it is better to just SteamRoot, as this compensates for these anomalies
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

    /// The steamapps folder from this steam library
    pub fn get_steamapps_folder(&self) -> PathBuf {
        self.steamapps.clone()
    }

    /// If this is the root library (and only if),
    /// then you will be able to retrieve the Steamroot from it again
    pub fn convert_to_steamroot(&self) -> Option<SteamRoot> {
        if self.is_root {
            let mut folder = self.steamapps.clone();
            folder.pop();
            steam_root_from(folder)
        } else {
            None
        }
    }

    /// Retruns if this Library is the one and only root library
    pub fn is_root(&self) -> bool {
        self.is_root
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

/// Returns the steam root from the path set in env `$STEAM_DIR`
///
/// This will still return the value, even if `no_tricks` is set
/// If no path was set returns Err(false)
/// If the value set was not a valid path (or did not exist) returns Err(true)
pub fn steam_root_env() -> Result<SteamRoot, bool> {
    steam_root_from(get_steam_dir_env_path()?).ok_or(true)
}

// Common Steam Paths
const STEAM_DOT_STEAM: &str = "~/.steam/steam";
const STEAM_LOCAL_SHARE: &str = "~/.local/share/Steam";
const STEAM_FLATPAK: &str = "~/.var/app/com.valvesoftware.Steam/data/Steam/";

/// Returns the first steam root found.
///
/// The Result indicates if an invalid `STEAM_DIR` was set (if you set `no_tricks` you can disgard
/// all Err, it will always return Ok). It return Ok on unset `STEAM_DIR`
/// Err(Some(other)) indicates another standard steam root was found, Err(None) that none.
/// Similarly, Ok(Some(root)) indicates the a root was found (this is the first one), Ok(None) indicates no root was found
///
/// The order in which steam roots are found is:
/// - $STEAM_DIR (skipped if `no_tricks`)
/// - ~/.steam/steam/
/// - ~/.local/share/steam/
/// - ~/.var/app/com.valvesoftware.Steam/data/Steam/
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
///
/// The Result indicates if an invalid `STEAM_DIR` was set (if you set `no_tricks` you can disgard
/// all Err, it will always return Ok). It return Ok on unset `STEAM_DIR`
/// If ~/.steam/steam symlinks to ~/.local/share/steam/ then the path will be included only once
///
/// The order in which steam roots are found is:
/// - $STEAM_DIR (skipped if `no_tricks`)
/// - ~/.steam/steam/
/// - ~/.local/share/steam/
/// - ~/.var/app/com.valvesoftware.Steam/data/Steam/
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
#[derive(Debug, Clone)]
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

    /// Returns the prefix folder,
    /// containing the dosdevices folder, and registry files
    pub fn get_pfx_path(&self) -> PathBuf {
        self.pfx.clone()
    }

    /// Returns what is treated as the C drive within the prefix
    pub fn get_c_drive(&self) -> PathBuf {
        self.parse_windows_path("C:\\")
    }

    /// Returns the home folder for the user within the prefix,
    /// usually `C:\Users\username`
    pub fn home_dir(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_VOLATILE, "USERPROFILE")
    }

    /// Returns the AppData\Roaming folder within the Home folder within the prefix
    pub fn appdata_roaming(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_SHELL_FOLDERS, "AppData")
    }

    /// Returns the AppData\Local folder within the Home folder within the prefix
    pub fn appdata_local(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_SHELL_FOLDERS, "Local AppData")
    }

    /// Returns the AppData\LocalLow folder within the Home folder within the prefix
    pub fn appdata_local_low(&self) -> Option<PathBuf> {
        // Yeah, this is the key of hell... but if it works...
        self.get_path_from_registry(REG_SHELL_FOLDERS, "{A520A1A4-1780-4FF6-BD18-167343C5AF16}")
    }

    /// Returns the Music folder within the Home folder within the prefix
    pub fn music_dir(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_SHELL_FOLDERS, "My Music")
    }

    /// Returns the Videos folder within the Home folder within the prefix
    pub fn videos_dir(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_SHELL_FOLDERS, "My Videos")
    }

    /// Returns the Pictures folder within the Home folder within the prefix
    pub fn picture_dir(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_SHELL_FOLDERS, "My Pictures")
    }

    /// Returns the Documents folder within the Home folder within the prefix
    pub fn documents_dir(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_SHELL_FOLDERS, "Personal")
    }

    /// Returns the Downloads folder within the Home folder within the prefix
    pub fn downloads_dir(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_SHELL_FOLDERS, "{374DE290-123F-4565-9164-39C4925E467B}")
    }

    /// Returns the Desktop folder within the Home folder within the prefix
    pub fn desktop_dir(&self) -> Option<PathBuf> {
        self.get_path_from_registry(REG_SHELL_FOLDERS, "Desktop")
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

    /// Returns the public user folder within the prefix
    pub fn public_user_dir(&self) -> Option<PathBuf> {
        let mut user_reg = self.pfx.clone();
        user_reg.push("system.reg");
        if let Some(user_reg) = RegParser::new(user_reg) {
            if let Some(val) = user_reg.open_key("Software\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList") {
                if let Some(path) = val.get("Public") {
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
///
/// There is a chance there are multiple prefixes through multiple steam installs.
/// The Result indicates if an invalid `STEAM_DIR` was set (if you set `no_tricks` you can disgard
/// all Err, it will always return Ok). It returns Ok on unset `STEAM_DIR`
/// Ok(Some(prefix)) indicates the prefix for the game was found, but not necessarily within the
/// set `STEAM_DIR`.
///
/// The search order is:
/// - $STEAM_DIR (skipped if `no_tricks`)
/// - ~/.steam/steam/
/// - ~/.local/share/steam/
/// - ~/.var/app/com.valvesoftware.Steam/data/Steam/
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
///
/// There is a chance there are multiple prefixes through multiple steam installs.
/// The Result indicates if an invalid `STEAM_DIR` was set (if you set `no_tricks` you can disgard
/// all Err, it will always return Ok). It returns Ok on unset `STEAM_DIR`
///
/// The search order is:
/// - $STEAM_DIR (skipped if `no_tricks`)
/// - ~/.steam/steam/
/// - ~/.local/share/steam/
/// - ~/.var/app/com.valvesoftware.Steam/data/Steam/
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
#[derive(Debug)]
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
