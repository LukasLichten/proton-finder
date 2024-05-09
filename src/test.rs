
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
fn get_prefix(game_id: u32) -> crate::linux::ProtonPrefix {
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
    let path = get_prefix(2420510).get_c_drive();
    assert!(path.is_dir(), "Unable to find HoloCure drive_c folder: {}", path.to_str().unwrap());
}

#[cfg(target_os = "linux")]
#[test]
pub fn find_holocure_home() {
    // We test if we can find the home folder of the user within the prefix for the game HoloCure
    //
    // This test fails if steam and HoloCure have not been installed and initialized
    let path = get_prefix(2420510).home();
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
    let path = get_prefix(2420510).appdata_roaming();
    assert!(path.is_some(), "Unable to find HoloCure user appdata/roaming folder: None returned");
    let path = path.unwrap();
    assert!(path.is_dir(), "Unable to find HoloCure user appdata/roaming folder: {}", path.to_str().unwrap());
}
