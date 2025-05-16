<div align="center">
  <img src="https://francescobbo.github.io/cruStation/logo-gh.svg" alt="cruStation" width="200" height="200">

  <h3 align="center">cruStation</h3>

  <p align="center">
    A PlayStation (PS1) emulator written in Rust.
  </p>
</div>

## 🌟 Features

* **CPU Emulation**: emulates the PlayStation's main R3000A-compatible CPU.
* **GPU Emulation**: handles some graphics rendering. Rendering is done using the `wgpu` library (could it even run in a browser?).
* **Debugging**: some debugging and stepping features are available.

It cannot currently run any games, since the CD-ROM drive is not emulated yet. However, the BIOS boot scene is completely displayed.

![cruStation](/boot.png)

## 🛠️ Getting Started

### Prerequisites

* **Rust**: Ensure you have a recent version of Rust installed. You can get it from [rust-lang.org](https://www.rust-lang.org/).
* **PlayStation BIOS**: You will need a PlayStation BIOS ROM image (e.g., `SCPH1001.BIN`). Place it in a `bios/` directory in the project root.

### Building

1.  Clone the repository:
    ```bash
    git clone <your-repository-url>
    cd crustation
    ```
2.  Build the project using Cargo:
    ```bash
    cargo build --release
    ```

### Running

* To run the emulator (it will attempt to load the BIOS):
    ```bash
    cargo run --release
    ```
* To run a specific PlayStation executable:
    ```bash
    cargo run --release path/to/your/executable.exe
    ```
    (The emulator loads the BIOS first, then the executable.)

## ⚙️ Dependencies

* `env_logger` (or a similar logging facade)
* `pollster`
* `crustationgui` (the GUI crate for this project)
    * `winit` (for window creation and event handling)
    * `wgpu` (or a library wrapping it, for graphics rendering)
    * `crossbeam-channel` (for communication, between CPU and GPU threads)
* `ctrlc` (for handling Ctrl-C interrupts)
* Standard Rust libraries.

## 📜 License

This project is licensed under the MIT License.