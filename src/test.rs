
#[cfg(target_os = "linux")]
#[test]
pub fn find_steam_root() {
    // We test if a steam root can be found at all
    // As such we ignore errors from incorrect $STEAM_DIR
    //
    // This test fails if steam is not installed
    assert!(match crate::linux::find_steam_root() {
        Ok(res) => res,
        Err(res) => res
    }.is_some(), "Unable to find any steam install");
    
}

#[cfg(target_os = "linux")]
fn find_prefix_helper(game_id: u32) -> crate::linux::ProtonPrefix {
    let prefix = match crate::linux::find_prefix(game_id) {
        Ok(res) => res,
        Err(res) => res
    };

    assert!(prefix.is_some(), "Unable to find prefix for game {}", game_id);
    prefix.unwrap()
}

#[cfg(target_os = "linux")]
#[test]
pub fn find_holocure_prefix() {
    // We test if we can find the prefix for the game HoloCure
    //
    // This test fails if steam and HoloCure have not been installed and initialized
    assert!(match crate::linux::find_prefix(2420510) {
        Ok(res) => res,
        Err(res) => res
    }.is_some(), "Unable to find HoloCure Prefix (2420510)");
}


#[cfg(target_os = "linux")]
#[test]
pub fn find_holocure_c_drive() {
    // We test if we can find the c_drive within the prefix for the game HoloCure
    //
    // This test fails if steam and HoloCure have not been installed and initialized
    let path = find_prefix_helper(2420510).get_c_drive();
    assert!(path.is_dir(), "Unable to find HoloCure drive_c folder: {}", path.to_str().unwrap());
}

#[cfg(target_os = "linux")]
#[test]
pub fn find_holocure_home() {
    // We test if we can find the home folder of the user within the prefix for the game HoloCure
    //
    // This test fails if steam and HoloCure have not been installed and initialized
    let path = find_prefix_helper(2420510).home_dir();
    assert!(path.is_some(), "Unable to find HoloCure user home folder: None returned");
    let path = path.unwrap();
    assert!(path.is_dir(), "Unable to find HoloCure user home folder: {}", path.to_str().unwrap());
}

#[cfg(target_os = "linux")]
#[test]
pub fn find_holocure_appdata_roaming() {
    // We test if we can find the home folder of the user within the prefix for the game HoloCure
    //
    // This test fails if steam and HoloCure have not been installed and initialized
    let path = find_prefix_helper(2420510).appdata_roaming();
    assert!(path.is_some(), "Unable to find HoloCure user appdata/roaming folder: None returned");
    let path = path.unwrap();
    assert!(path.is_dir(), "Unable to find HoloCure user appdata/roaming folder: {}", path.to_str().unwrap());
}

#[test]
pub fn universal_find_holocure_gamedrive() {
    // This test can never fail under windows
    // But it can fail under Linux if HoloCure is not installed (like for the tests above)

    let game_drive = match crate::get_game_drive(2420510) {
        Ok(res) => res,
        Err(res) => res
    };

    assert!(game_drive.is_some(), "Unable to find windows enviroment for game HoloCure (2420510)");
}

fn get_game_drive_helper(game_id: u32) -> crate::GameDrive {
    let prefix = match crate::get_game_drive(game_id) {
        Ok(res) => res,
        Err(res) => res
    };

    assert!(prefix.is_some(), "Unable to find windows enviroment for game {}", game_id);
    prefix.unwrap()
}


#[test]
pub fn universal_find_holocure_c_drive() {
    // This test can never fail under windows
    // But it can fail under Linux if HoloCure is not installed (like for the tests above)

    let path = get_game_drive_helper(2420510).c_drive();
    assert!(path.is_dir(), "C drive within enviroment for HoloCure not found: {}", path.to_str().unwrap());
}

#[test]
pub fn universal_find_holocure_user_folder() {
    // This test can fail under Windows (but incredibly unlikely)
    // But it can fail under Linux if HoloCure is not installed (like for the tests above)

    let path = get_game_drive_helper(2420510).home_dir();
    assert!(path.is_some(), "Home folder within enviroment for HoloCure not found: None returned");
    let path = path.unwrap();
    assert!(path.is_dir(), "Home folder within enviroment for HoloCure not found: {}", path.to_str().unwrap());
}

#[test]
pub fn universal_find_holocure_public() {
    // This test can fail under Windows (but incredibly unlikely)
    // But it can fail under Linux if HoloCure is not installed (like for the tests above)

    let path = get_game_drive_helper(2420510).public_dir();
    assert!(path.is_some(), "Public folder within enviroment for HoloCure not found: None returned");
    let path = path.unwrap();
    assert!(path.is_dir(), "Public folder within enviroment for HoloCure not found: {}", path.to_str().unwrap());
}

#[test]
pub fn universal_find_holocure_version_ini() {
    // This test can fail under Windows Linux if HoloCure is not installed and not launched at least once
    // (it could also break if the version.ini is removed by the HoloCure dev)

    let path = get_game_drive_helper(2420510).config_local_dir();
    assert!(path.is_some(), "AppData\\Local folder within enviroment for HoloCure not found: None returned");
    let mut path = path.unwrap();
    assert!(path.is_dir(), "AppData\\Local within enviroment for HoloCure not found: {}", path.to_str().unwrap());
    path.push("HoloCure");
    assert!(path.is_dir(), "HoloCure save folder within enviroment for HoloCure not found: {}", path.to_str().unwrap());
    path.push("version.ini");
    assert!(path.is_file(), "HoloCure save version.ini file within enviroment for HoloCure not found: {}", path.to_str().unwrap());
}

// fn example() {
//     let res = crate::get_game_drive(2420510).map_or_else(|e| {
//         println!("Steam Dir provided is not correctly formated, ignored...");
//         e
//     }, |k| k);
//
//     if let Some(game_drive) = res {
//         if let Some(mut path) = game_drive.config_local_dir() {
//             path.push("HoloCure");
//             path.push("settings.json");
//             if let Ok(text) = std::fs::read_to_string(path) {
//                 println!("{}", text);
//             }
//         } else {
//             println!("Unable to find AppData\\Local (Something has gone horribly wrong)")
//         }
//     } else {
//         println!("Unable to find game drive. Did you install the game?");
//     }
// }
