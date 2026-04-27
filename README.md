# Tharsis

![Tharsis Hero](https://images.unsplash.com/photo-1614850523296-d8c1af93d400?auto=format&fit=crop&q=80&w=1200)

**Tharsis** is a high-performance, multi-segmented download manager built with **Tauri v2**, **Rust**, and **React**. Designed for speed, reliability, and a premium user experience, Tharsis leverages segmented downloading to saturate your bandwidth and provides a sleek, glassmorphic interface for managing your files.

## 🚀 Key Features

- **Multi-Segmented Downloads**: Automatically splits files into multiple chunks (default: 4) to accelerate download speeds.
- **SQLite Persistence**: Your download history and progress are saved in a local SQLite database, ensuring persistence across restarts.
- **Glassmorphic UI**: A stunning, modern interface built with Tailwind CSS and Shadcn UI, featuring backdrop blurs, gradients, and micro-animations.
- **Live Progress Tracking**: Real-time updates for download speed, percentage, and total bytes via Tauri's high-speed IPC bridge.
- **WAL-Enabled Database**: Optimized SQLite configuration (Write-Ahead Logging) to handle high-frequency progress updates without UI stuttering.
- **Cross-Platform**: Built on Tauri v2, ready for Windows, macOS, and Linux.

## 🛠 Tech Stack

- **Backend**: [Rust](https://www.rust-lang.org/)
    - [Tauri v2](https://v2.tauri.app/): Framework for building tiny, fast binaries.
    - [tokio](https://tokio.rs/): Asynchronous runtime.
    - [reqwest](https://docs.rs/reqwest/): HTTP client for segmented requests.
    - [sqlx](https://github.com/launchbadge/sqlx): Async SQLite driver with compile-time checked queries.
- **Frontend**: [React](https://reactjs.org/)
    - [TypeScript](https://www.typescriptlang.org/): For type-safe UI logic.
    - [Tailwind CSS](https://tailwindcss.com/): Utility-first styling.
    - [Lucide React](https://lucide.dev/): Beautiful, consistent iconography.

## 🏗 Architecture

Tharsis follows a decoupled architecture where the Rust backend handles the heavy lifting of network I/O and file system operations, while the React frontend provides a responsive interface.

### Segmented Download Engine
The core engine (`engine.rs`) utilizes `reqwest`'s `Range` headers to request specific byte ranges of a file simultaneously. Each chunk is downloaded in its own `tokio` task. Once all chunks are complete, they are stitched together into the final file, and the temporary `.part` files are cleaned up.

### Database Strategy
To prevent I/O contention, Tharsis uses SQLite in **WAL (Write-Ahead Logging)** mode. Progress updates are throttled (both in the DB and UI) to maintain 60FPS UI performance even during multi-gigabit downloads.

## 📥 Installation

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) (v18+)
- [Tauri CLI](https://v2.tauri.app/reference/cli/)

### Setup
1. Clone the repository:
   ```bash
   git clone https://github.com/user/tharsis.git
   cd tharsis
   ```
2. Install dependencies:
   ```bash
   npm install
   ```
3. Run in development mode:
   ```bash
   npm run tauri dev
   ```

## 📂 Project Structure

```text
tharsis/
├── src/                # React Frontend
│   ├── components/     # UI Components (Shadcn)
│   ├── App.tsx         # Main UI Logic & IPC Listener
│   └── index.css       # Global Styles & Glassmorphism
├── src-tauri/          # Rust Backend
│   ├── src/
│   │   ├── engine.rs   # Multi-threaded Download Core
│   │   ├── db.rs       # SQLite Init & WAL Config
│   │   ├── queries.rs  # SQLx Database Queries
│   │   └── lib.rs      # Tauri Command Handlers
│   └── Cargo.toml      # Rust Dependencies
└── migrations/         # SQL Schema Migrations
```

## ⚖ License

Tharsis is released under the [MIT License](LICENSE).
