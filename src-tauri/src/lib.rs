use tauri::Emitter;
use tauri_plugin_store::StoreExt;
use serde_json;
use serde::Serialize;

struct ChapterTitle {
    timestamp: i32,
    title: String,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    current: usize,
    total: usize,
    title: String,
}

#[derive(Debug, serde::Serialize)]
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
    }
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
        AppError::Vimeo { message: message.to_string() }
    }

    fn rate_limit(message: &str, reset_time: &str) -> AppError {
        AppError::RateLimit {
            message: message.to_string(),
            reset_time: reset_time.to_string(),
        }
    }
}

#[tauri::command]
async fn upload_chapter_titles(video_id: &str, text: &str, offset: &str, handle: tauri::AppHandle) -> Result<(), AppError> {
    let offset = if offset.trim().is_empty() {
        0
    } else {
        parse_timestamp(offset).map_err(|e| AppError::parse(&format!("Invalid offset ({})", e), 0, offset))?
    };

    // Parse chapters and apply offset
    let mut chapters = parse_chapter_titles(text)?;

    chapters = chapters
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
    let total = chapters.len();

    for (i, chapter) in chapters.into_iter().enumerate() {
        // Send progress to frontend
        handle.emit("upload-progress", ProgressPayload {
            current: i + 1,
            total,
            title: chapter.title.clone(),
        }).unwrap();

        let response = client
            .post(format!("https://api.vimeo.com/videos/{}/chapters", video_id))
            .bearer_auth(&access_token)
            .json(&serde_json::json!({
                "timecode": chapter.timestamp,
                "title": chapter.title,
            }))
            .send()
            .await
            .map_err(|e| AppError::vimeo(&format!("Vimeo Issue ({e})")))?;

        // If request gets a bad response, give error
        if !response.status().is_success() {
            let status = response.status().as_u16();

            match status {
                401 => return Err(AppError::auth("Access token is invalid or expired.")),

                403 => return Err(AppError::vimeo("You don't have permission to edit this video.")),

                404 => return Err(AppError::vimeo("Couldn't find video, double check the Video ID.")),

                429 => {
                    let reset_time = response.headers()
                        .get("X-RateLimit-Reset")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("unkown");

                    return Err(AppError::rate_limit("You've been rate limited.", reset_time));
                },

                500..=599 => return Err(AppError::vimeo("Vimeo is currenlty experiencing server issues. Please try again later.")),

                _ => {
                    let body: serde_json::Value = response.json().await.unwrap_or_default();
                    let msg = body["developer_message"].as_str().unwrap_or("Unknown Vimeo API error.");
                    
                    return Err(AppError::vimeo(msg))
                }
            }
        }
    }

    
    
    Ok(())
}

#[tauri::command]
async fn get_default_offset(text: &str) -> Result<String, AppError> {
    text
        .lines()
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
        .map(|timestamp| format!("{:02}:{:02}:{:02}", timestamp / 3600, (timestamp % 3600) / 60, timestamp % 60))
}

fn parse_chapter_titles(text: &str) -> Result<Vec<ChapterTitle>, AppError> {
    text
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let line = line.trim();
            // If line is empty, return Ok(None) but don't error
            if line.is_empty() {
                return Ok(None);
            }

            // Wrap each successful parse and error in Some
            parse_chapter_title(line).map(Some).map_err(|err| AppError::Parse {
                message: err,
                line_number: i,
                raw_line: line.to_string(),
            })
        })
        .filter_map(|res| res.transpose()) // Swap the Res<Option> to an Option<Res> and the gets rid of the Option
        .collect()
}

fn parse_chapter_title(line: &str) -> Result<ChapterTitle, String> {
    let (timestamp, title) = line.split_once('-')
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
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            upload_chapter_titles,
            get_default_offset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
