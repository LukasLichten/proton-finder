use std::path::PathBuf;

/// These are more direct bindings for Linux specifically
#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(test)]
pub mod test;

/// An Abstraction for ProtonPrefix under Linux/Native Filesystem under Windows
///
/// Under Windows, this is a wrapper for dirs
/// Under Linux, it will return you the same paths within the prefix as dirs would
pub struct GameDrive {
    #[cfg(target_os = "linux")]
    prefix: linux::ProtonPrefix
}

/// Retrives the abstraction for the access to common folders
///
/// Under Windows this will always return Ok(Some)
///
/// Under Linux the user can pass in `$STEAM_DIR` (except if `no_tricks` is set).
/// If this env is set but the path is invalid/doesn't point to a steam installation, then this
/// function will return an Err (so if `no_tricks` is set, this function can not Err).
/// Some is returned if the proton prefix was found, but this doesn't have to be from the
/// `$STEAM_DIR`, as it will search through all till it finds one.
/// Search order:
/// $STEAM_DIR (skipped if `no_tricks` or unset)
/// ~/.steam/steam/
/// ~/.local/share/steam/
/// ~/.var/app/com.valvesoftware.Steam/data/Steam/
///
/// Under unsupported plattforms this return Ok(None)
///
/// So appropriate error handling is map_or_else the Result, throw an error message to the user for
/// err (that the path he defined in `$STEAM_DIR` is invalid), but then you can continue with the Option.
/// The Option you handle with an error message and shutdown on none (that the game needs to be
/// installed and launched once for the prefix to exist)
pub fn get_game_drive(game_id: u32) -> Result<Option<GameDrive>, Option<GameDrive>> {
    #[cfg(target_os = "windows")]
    {
        return Ok(Some(GameDrive { }));
    }

    #[cfg(target_os = "linux")]
    {
        let (res, err) = match linux::find_prefix(game_id) {
            Ok(res) => (res, false),
            Err(res) => (res, true)
        };

        let res = if let Some(prefix) = res {
            Some(GameDrive { prefix })
        } else {
            None
        };

        return match err {
            false => Ok(res),
            true => Err(res)
        };
    }
    

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        return Ok(None);
    }
}

impl GameDrive {

    /// Returns the path to the C Drive
    pub fn c_drive(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        return PathBuf::from_str("C:\\");

        #[cfg(target_os = "linux")]
        return self.prefix.get_c_drive();

        
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        PathBuf::new()
    }

    // This works like `dirs::home_dir` under Windows would
    // returning `C:\Users\%username%`
    pub fn home_dir(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        return dirs::home_dir();

        #[cfg(target_os = "linux")]
        return self.prefix.home_dir();

        
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        None
    }

    // This works like `dirs::data_dir` under Windows would
    // returning `C:\Users\%username%\AppData\Roaming`
    pub fn data_dir(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        return dirs::data_dir();

        #[cfg(target_os = "linux")]
        return self.prefix.home_dir();

        
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        None
    }

}
