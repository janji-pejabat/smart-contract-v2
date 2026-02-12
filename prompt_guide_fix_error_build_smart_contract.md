# 🛠️ Prompt Guide: Fix CosmWasm Build Errors (Paxi Network Edition)

Gunakan panduan ini jika Anda menemui error `the lock file Cargo.lock needs to be updated` atau kegagalan validasi WASM saat deploy ke **Paxi Network**.

## 📝 Masalah Umum (Problem Analysis)
1. **Missing `Cargo.lock`**: Docker Optimizer menjalankan build dengan flag `--locked`. Jika `Cargo.lock` tidak ada di repo (biasanya ter-ignore di `.gitignore`), build akan langsung gagal (Exit Code 101).
2. **Opcodes Tidak Kompatibel**: Paxi Network hanya menerima WASM versi **MVP**. Compiler Rust terbaru sering menghasilkan opcode modern (`memory.copy`, `sign-extension`) yang akan ditolak oleh chain.
3. **Workspace Inconsistent**: Project dengan banyak kontrak butuh root `Cargo.toml` agar optimizer bisa bekerja efisien dan konsisten.

---

## 🤖 Prompt untuk AI Agent (Jules/GPT)

Copy-paste prompt di bawah ini ke AI Agent Anda untuk memperbaiki repository secara otomatis:

> "Saya memiliki masalah build pada project CosmWasm ini. Tolong perbaiki dengan langkah-langkah berikut:
>
> 1. **Cargo Workspace**: Jika belum ada, buat root `Cargo.toml` sebagai virtual workspace yang membungkus semua contract di folder `contracts/`. Pastikan `resolver = "2"` dan tambahkan `[profile.release]` dengan `opt-level = 3` dan `overflow-checks = true`.
>
> 2. **Cargo.lock Fix**: Hapus `Cargo.lock` dari file `.gitignore`. Jalankan `cargo update` di root untuk menghasilkan lock file yang sinkron dengan workspace, lalu pastikan file tersebut siap di-commit.
>
> 3. **Paxi Compatibility**: Tambahkan file `.cargo/config.toml` di root dengan konfigurasi rustflags:
>    ```toml
>    [target.wasm32-unknown-unknown]
>    rustflags = [
>      "-C", "target-cpu=mvp",
>      "-C", "target-feature=-bulk-memory",
>      "-C", "target-feature=-sign-ext",
>      "-C", "link-arg=-s",
>    ]
>    ```
>
> 4. **Official Optimizer**: Update script `build_all.sh` (atau buat baru) untuk menggunakan Docker image `cosmwasm/workspace-optimizer:0.16.1`. Ini adalah satu-satunya cara yang dijamin 100% menghasilkan WASM MVP yang valid untuk Paxi Network.
>
> 5. **CI/CD Alignment**: Pastikan `.github/workflows/build.yml` menggunakan Docker optimizer yang sama untuk build produksinya.
>
> 6. **Verification**: Jalankan `cargo test --workspace` untuk memastikan tidak ada perubahan struktur yang merusak logika contract.
>
> Tolong handle semuanya sampai project ini bisa di-build dengan lancar menggunakan command:
> `docker run --rm -v \"$(pwd)\":/code cosmwasm/workspace-optimizer:0.16.1`"

---

## 🛠️ Langkah Manual (Jika Ingin Fix Sendiri)

1. **Pastikan `Cargo.lock` di-commit**:
   - `git rm --cached Cargo.lock` (jika terlanjur di-track tapi salah)
   - Edit `.gitignore`, hapus baris `Cargo.lock`.
   - `cargo build`
   - `git add Cargo.lock`

2. **Struktur Workspace**:
   Root `Cargo.toml` harus terlihat seperti ini:
   ```toml
   [workspace]
   members = ["contracts/*"]
   resolver = "2"

   [profile.release]
   opt-level = 3
   debug = false
   rpath = false
   lto = true
   debug-assertions = false
   codegen-units = 1
   panic = 'abort'
   incremental = false
   overflow-checks = true
   ```

3. **Gunakan Optimizer yang Tepat**:
   Selalu gunakan versi `0.16.1` (atau terbaru yang stabil) untuk Paxi Network agar translasi opcode modern ke MVP ditangani secara otomatis oleh optimizer.
