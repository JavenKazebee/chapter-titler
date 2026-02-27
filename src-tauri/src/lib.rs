use serde::Serialize;
use serde_json;
use tauri::{Emitter, Manager};
use tauri_plugin_store::StoreExt;

struct ChapterTitle {
    timestamp: i32,
    title: String,
}

#[derive(Serialize, Clone)]
struct UploadResult {
    successful: usize,
    total: usize,
    error: Option<AppError>,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    current: usize,
    total: usize,
    title: String,
}

#[derive(Debug, serde::Serialize, Clone)]
#[serde(tag = "type", content = "data")]
enum AppError {
    Parse {
        message: String,
        line_number: usize,
        raw_line: String,
    },
    Auth {
        message: String,
    },
    Vimeo {
        message: String,
    },
    RateLimit {
        message: String,
        reset_time: String,
    },
    Offset {
        message: String,
    },
}

impl AppError {
    fn parse(message: &str, line_number: usize, raw_line: &str) -> AppError {
        AppError::Parse {
            message: message.to_string(),
            line_number,
            raw_line: raw_line.to_string(),
        }
    }

    fn auth(message: &str) -> AppError {
        AppError::Auth {
            message: message.to_string(),
        }
    }

    fn vimeo(message: &str) -> AppError {
        AppError::Vimeo {
            message: message.to_string(),
        }
    }

    fn rate_limit(message: &str, reset_time: &str) -> AppError {
        AppError::RateLimit {
            message: message.to_string(),
            reset_time: reset_time.to_string(),
        }
    }

    fn offset(message: &str) -> AppError {
        AppError::Offset {
            message: message.to_string(),
        }
    }
}

#[tauri::command]
async fn upload_chapter_titles(
    video_id: &str,
    text: &str,
    offset: &str,
    start_index: usize,
    handle: tauri::AppHandle,
) -> Result<UploadResult, AppError> {
    let mut upload_result = UploadResult {
        successful: 0,
        total: 0,
        error: None,
    };

    // Parse offset
    let offset = if offset.trim().is_empty() {
        0
    } else {
        parse_timestamp(offset).map_err(|e| AppError::offset(&format!("Invalid offset ({})", e)))?
    };

    // Parse chapters and apply offset
    let chapters = parse_chapter_titles(text)?;

    let chapters: Vec<ChapterTitle> = chapters
        .into_iter()
        .filter_map(|mut chapter| {
            let new_timestamp = chapter.timestamp - offset;

            // Filter out negative timestamps
            if new_timestamp >= 0 {
                chapter.timestamp = new_timestamp;
                Some(chapter)
            } else {
                None
            }
        })
        .collect();

    // Store total number of chapters before applying start index
    upload_result.total = chapters.len();
    upload_result.successful = start_index;

    // Apply start index
    let chapters: Vec<ChapterTitle> = chapters.into_iter().skip(start_index).collect();

    if chapters.is_empty() {
        return Err(AppError::parse(
            "No chapters found. Make sure you have at least one chapter after the offset.",
            0,
            text,
        ));
    }

    // Load access_token
    let store = handle
        .store("data.json")
        .map_err(|e| AppError::auth(&format!("Failed to load authentication data file. ({e})")))?;
    let access_token = store
        .get("access_token")
        .and_then(|val| val.as_str().map(|s| s.to_string()))
        .ok_or_else(|| AppError::auth("Couldn't find Access Token."))?;

    // Upload chapters
    let client = reqwest::Client::new();

    for (i, chapter) in chapters.into_iter().enumerate() {
        // Send progress to frontend
        handle
            .emit(
                "upload-progress",
                ProgressPayload {
                    current: i + 1,
                    total: upload_result.total,
                    title: chapter.title.clone(),
                },
            )
            .unwrap();

        println!("Uploading chapter {} of {} ({} - {})", i + 1, upload_result.total, chapter.timestamp, chapter.title);
        let response = client
            .post(format!(
                "https://api.vimeo.com/videos/{}/chapters",
                video_id
            ))
            .bearer_auth(&access_token)
            .json(&serde_json::json!({
                "timecode": chapter.timestamp,
                "title": chapter.title,
            }))
            .send()
            .await;

        match response {
            Ok(response) => {
                if response.status().is_success() {
                    upload_result.successful += 1;
                } else {
                    let status = response.status().as_u16();

                    match status {
                        400 => {
                            let body: serde_json::Value = response.json().await.unwrap_or_default();
    
                            // Try to get the specific reason from the first invalid parameter
                            let specific_reason = body["invalid_parameters"]
                                .as_array()
                                .and_then(|arr| arr.get(0))
                                .and_then(|param| {
                                    let field = param["field"].as_str().unwrap_or("unknown field");
                                    let reason = param["reason"].as_str().unwrap_or("invalid value");
                                    Some(format!("{}: {}", field, reason))
                                })
                                .unwrap_or_else(|| "Invalid request data".to_string());

                            upload_result.error = Some(AppError::vimeo(&format!("Validation Error ({})", specific_reason)))
                        }

                        401 => upload_result.error = Some(AppError::auth("Access token is invalid or expired.")),

                        403 => {
                            upload_result.error = Some(AppError::vimeo(
                                "You don't have permission to edit this video.",
                            ))
                        }

                        404 => {
                            upload_result.error = Some(AppError::vimeo(
                                "Couldn't find video, double check the Video ID.",
                            ))
                        }

                        429 => {
                            let reset_time = response
                                .headers()
                                .get("X-RateLimit-Reset")
                                .and_then(|h| h.to_str().ok())
                                .unwrap_or("unkown");

                            upload_result.error = Some(AppError::rate_limit(
                                "You've been rate limited.",
                                reset_time,
                            ));
                        }

                        500..=599 => {
                            upload_result.error = Some(AppError::vimeo(
                                "Vimeo is currently experiencing server issues. Please try again later.",
                            ))
                        }

                        _ => {
                            let body: serde_json::Value = response.json().await.unwrap_or_default();

                            let msg = body["developer_message"]
                                .as_str()
                                .unwrap_or("Unknown Vimeo API error.");

                            upload_result.error = Some(AppError::vimeo(&format!("{} - {}", status, msg)));
                        }
                    }
                }
            }
            Err(e) => {
                upload_result.error = Some(AppError::vimeo(&format!("Vimeo Issue ({e})")));
            }
        }

        // Stop uploading if an error occurs
        if upload_result.error.is_some() {
            break;
        }
    }

    Ok(upload_result)
}

#[tauri::command]
async fn get_default_offset(text: &str) -> Result<String, AppError> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            match parse_chapter_title(line) {
                Ok(chapter) => Some(Ok(chapter.timestamp)),
                Err(err) => Some(Err(AppError::parse(&err, 0, line))),
            }
        })
        .nth(1)
        .unwrap_or(Err(AppError::parse("No chapters found.", 0, "")))
        .map(|timestamp| {
            format!(
                "{:02}:{:02}:{:02}",
                timestamp / 3600,
                (timestamp % 3600) / 60,
                timestamp % 60
            )
        })
}

fn parse_chapter_titles(text: &str) -> Result<Vec<ChapterTitle>, AppError> {
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            let line = line.trim();
            // If line is empty, return Ok(None) but don't error
            if line.is_empty() {
                return Ok(None);
            }

            // Wrap each successful parse and error in Some
            parse_chapter_title(line)
                .map(Some)
                .map_err(|err| AppError::Parse {
                    message: err,
                    line_number: i,
                    raw_line: line.to_string(),
                })
        })
        .filter_map(|res| res.transpose()) // Swap the Res<Option> to an Option<Res> and the gets rid of the Option
        .collect()
}

fn parse_chapter_title(line: &str) -> Result<ChapterTitle, String> {
    let (timestamp, title) = line
        .split_once('-')
        .ok_or_else(|| "Missing hyphen '-', between timestamp and title")?;

    let timestamp = parse_timestamp(timestamp.trim())?;
    let title = title.trim().to_string();

    Ok(ChapterTitle {
        timestamp: timestamp,
        title: title,
    })
}

fn parse_timestamp(timestamp_str: &str) -> Result<i32, String> {
    let mut parts = timestamp_str.split(':').rev();

    let seconds: i32 = parts
        .next()
        .ok_or("Couldn't find seconds.")?
        .parse()
        .map_err(|_| "Seconds must be a number.")?;

    let minutes: i32 = parts
        .next()
        .ok_or("Couldn't find minutes.")?
        .parse()
        .map_err(|_| "Minutes must be a number.")?;

    let hours: i32 = parts
        .next()
        .map(|s| s.parse::<i32>())
        .transpose()
        .map_err(|_| "Hours must be a number.")?
        .unwrap_or(0);

    Ok((hours * 3600) + (minutes * 60) + seconds)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("No main window")
                .set_focus();
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            upload_chapter_titles,
            get_default_offset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
