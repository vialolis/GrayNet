use std::fs;
use std::path::Path;

fn main() {
    // Ensure an icon exists for tauri-build.
    let icon_path = Path::new("icons").join("icon.ico");
    if !icon_path.exists() {
        if let Some(parent) = icon_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Generate a minimal 16x16 icon.
        let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
        let image = ico::IconImage::from_rgba_data(16, 16, vec![0; 16 * 16 * 4]);
        icon_dir.add_entry(ico::IconDirEntry::encode(&image).expect("failed to encode icon"));
        let mut file = fs::File::create(&icon_path).expect("failed to create icon file");
        icon_dir.write(&mut file).expect("failed to write icon file");
    }

    tauri_build::build();
}
