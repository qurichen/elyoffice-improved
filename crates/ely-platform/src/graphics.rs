use anyhow::Result;

/// Fails when no graphics device exists; probed on macOS only.
pub fn check() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let devices = metal::Device::all();
        anyhow::ensure!(
            !devices.is_empty() || metal::Device::system_default().is_some(),
            "no Metal graphics device was found"
        );
        log::info!("graphics: {} Metal device(s)", devices.len());
    }
    #[cfg(not(target_os = "macos"))]
    log::info!("graphics: the backend is not probed on this OS");
    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn this_mac_has_a_metal_device() {
        super::check().unwrap();
    }
}
