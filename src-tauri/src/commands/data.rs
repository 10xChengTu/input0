use crate::errors::AppError;

#[tauri::command]
pub async fn export_data_to_file(data: String) -> Result<bool, AppError> {
    let file = rfd::AsyncFileDialog::new()
        .set_file_name("input0-data.json")
        .add_filter("JSON", &["json"])
        .save_file()
        .await;

    match file {
        Some(handle) => {
            std::fs::write(handle.path(), data.as_bytes())?;
            Ok(true)
        }
        None => Ok(false),
    }
}

#[tauri::command]
pub async fn import_data_from_file() -> Result<Option<String>, AppError> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()
        .await;

    match file {
        Some(handle) => {
            let content = std::fs::read_to_string(handle.path())?;
            Ok(Some(content))
        }
        None => Ok(None),
    }
}
