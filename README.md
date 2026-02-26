# Vimeo Chapter Titler
> **Sync YouTube-style chapters to your Vimeo videos in seconds.**

![GitHub release (latest by date)](https://img.shields.io/github/v/release/JavenKazebee/chapter-titler)
![Tauri Version](https://img.shields.io/badge/Tauri-2.0-blue)
![Rust](https://img.shields.io/badge/Rust-1.75+-orange)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Vimeo Chapter Titler is a desktop application built with **Tauri**, **Rust**, and **Vue 3** that automates the tedious process of manually entering timestamps and titles into Vimeo's interface.

## Features
* **YouTube Format Support:** Copy/paste chapters directly from YouTube descriptions.
* **Smart Parsing:** Handles various formats like `00:00 - Intro` or `01:23:45 - Chapter Title`.
* **Live Rate-Limit Tracking:** Built-in countdown timers handle Vimeo's API limits gracefully.
* **Real-time Progress:** See exactly which chapter is being uploaded.
* **Auto-Updates:** Stay current with the latest fixes via the integrated Tauri Updater.

---

## Getting Started

### Prerequisites
* **Vimeo Personal Access Token:** You need a token with `edit` and `video_files` scopes.
* **Node.js & Rust:** (Required only if building from source).

### Installation
1.  Download the latest release for your OS from the [Releases page](https://github.com/JavenKazebee/chapter-titler/releases).
2.  Install the `.msi` (Windows) or `.dmg` (macOS).
3.  Launch the app and enter your Vimeo Access Token in the settings.

---

## How to Use
1.  **Input Video ID:** Enter the numeric ID found in your Vimeo URL (e.g., `123456789`).
2.  **Add Chapters:** Paste your list into the text area. 
    * *Correct Format:* `MM:SS - Title`
3.  **Upload:** Click "Upload."
4.  **Monitor:** Watch the progress bar. If you hit a rate limit, the app will pause and show a countdown until you can try again.

## Development

If you want to contribute or build the app yourself:

```bash
# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build the production version
pnpm tauri build
```
---

## Contributing
Contributions, issues, and feature requests are welcome!
1. Fork the Project
2. Create your Feature Branch (git checkout -b feature/AmazingFeature)
3. Commit your Changes (git commit -m 'Add some AmazingFeature')
4. Push to the Branch (git push origin feature/AmazingFeature)
5. Open a Pull Request

---

## License
Distributed under the MIT License. See [LICENSE](./LICENSE) for more information.