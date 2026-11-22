# 🦀 Rust Migration — Catatan Induk untuk Semua Agentic AI

> **Status:** DRAFT — dokumen ini adalah *source of truth* untuk seluruh agen AI
> yang berpartisipasi dalam porting FrankenPHP dari Go ke Rust.
> **Terakhir diperbarui:** 2026-11-22
> **Aturan emas:** Jika dokumen ini bertentangan dengan instruksi ad-hoc,
> dokumen ini menang. Jika ragu, berhenti dan eskalasi (lihat §7).

---

## 1. MISI & PRINSIP NON-NEGOTIABLE

**Misi:** Mengganti seluruh lapisan Go pada FrankenPHP dengan Rust murni,
menyisakan hanya engine PHP (Zend) + shim C minimal, menghasilkan satu binary
statis yang kompatibel penuh dengan perilaku FrankenPHP saat ini.

Prinsip yang TIDAK BOLEH dilanggar oleh agen mana pun:

| # | Prinsip | Penjelasan |
|---|---------|------------|
| P1 | **Perilaku adalah kontrak.** | Suite test Go (`*_test.go`, `testdata/`) adalah oracle konformansi. Port tidak "selesai" sampai test setara lulus. Jangan pernah mengubah test untuk membuat kode baru lulus. |
| P2 | **Rust-first, bukan Rust-only.** | Yang tetap C: `frankenphp.c`, `emulate_php_cli.c`, shim Zend. Yang pindah ke Rust: HTTP server, worker/thread management, config, CLI, metrics, watcher, Mercure client, semua `internal/`. |
| P3 | **`unsafe` adalah utang.** | Setiap blok `unsafe` wajib punya `// SAFETY:` comment yang menjelaskan invarianta. Kode tanpa penjelasan = PR ditolak otomatis. |
| P4 | **Strangler pattern.** | Branch utama harus selalu bisa build & boot. Migrasi per-komponen di belakang feature flag / crate terpisah. Tidak ada big-bang rewrite. |
| P5 | **Jangan hapus kode Go sebelum parity.** | File Go hanya boleh dihapus pada Fase 7, setelah modul Rust penggantinya lulus 100% test oracle + benchmark regresi < 5%. |
| P6 | **Verifikasi, jangan asumsikan.** | Agen WAJIB membaca file aktual (`cat`/grep) sebelum mengedit. Dilarang mengarang API crate, nama fungsi C, atau struktur Zend. Kalau tidak yakin → cek header PHP / docs crate. |
| P7 | **Satu perubahan, satu alasan.** | Setiap commit/PR merujuk satu task ID (§4). Campur-aduk refactor+fitur dilarang. |
| P8 | **Kompatibilitas konfigurasi.** | Caddyfile, JSON config, env var (`FRANKENPHP_*`), dan opsi worker harus tetap valid tanpa perubahan user-facing. |

---

## 2. RULES OF ENGAGEMENT (aturan kerja agen AI)

### 2.1 Aturan hard (pelanggaran = rollback)

1. **Dilarang force-push** ke branch apa pun, terutama `main`/`dev`.
2. **Dilarang menyentuh** tanpa approval manusia: `LICENSE`, `SECURITY.md`,
   workflow `.github/workflows/release*`, signing keys, bagian alokasi Zend
   pada `frankenphp.c` (counterpart `zendalloc_gofuzz.go`).
3. **Dilarang menambah dependensi crate baru** tanpa entri di
   `docs/rust-migration/DEPENDENCIES.md` (nama, versi, lisensi, alasan,
   alternatif yang ditolak). Lisensi wajib kompatibel MIT/Apache-2.0/BSD/ISC;
   lisensi copyleft kuat (GPL/AGPL/NON-COMMERCIAL) ditolak.
4. **Dilarang menjalankan** perintah destruktif (`rm -rf`, `git reset --hard`
   atas kerja orang lain, push tag) tanpa konfirmasi manusia eksplisit.
5. **Dilarang mengklaim "selesai"** tanpa output verifikasi: hasil
   `cargo build`, `cargo clippy -- -D warnings`, `cargo test`, dan test oracle
   yang relevan ditempel pada laporan task.

### 2.2 Aturan kerja harian

6. **Selalu mulai dengan rekon status**: `git log --oneline -10`,
   `git status`, baca board task (`TASKS.md`), baru menulis kode.
7. **Klaim task sebelum mengerjakan** — tandai `[IN-PROGRESS @agen-id]` di
   `TASKS.md`. Satu task = satu agen. Dua agen pada satu task = deadlock review.
8. **TDD translatif dulu**: untuk tiap unit Go yang diport, tulis dulu test
   Rust yang merupakan terjemahan langsung test Go-nya (sumber: file
   `*_test.go` yang sama), biarkan merah, lalu implementasi.
9. **Ukuran chunk**: satu sesi agen ≤ 1 modul/crate kecil. Dilarang
   meminta/melakukan "port seluruh server.go sekaligus".
10. **Format & lint gate lokal**: `cargo fmt --check`,
    `cargo clippy -- -D warnings`; `rustfmt.toml` repo adalah hukum. Test
    thread-safety pakai `loom`/TSAN bila modul memakai concurrency manual.
11. **Bahasa commit**: Conventional Commits, prefix scope crate
    (contoh: `feat(frankenphp-engine): add worker request loop`).
12. **Dokumentasi berjalan**: setiap crate baru mendapat `README.md` ringkas +
    module-level rustdoc; keputusan arsitektur ditulis sebagai ADR singkat di
    `docs/rust-migration/adr/NNN-*.md` (konteks → keputusan → konsekuensi).

### 2.3 Aturan spesifik FFI / PHP (paling berisiko)

13. **FFI hanya lewat crate `frankenphp-sys`.** Crate lain DILARANG
    mendeklarasikan `extern "C"` ke simbol Zend/PHP secara langsung.
14. **Binding dihasilkan, bukan ditulis tangan** — gunakan `bindgen` terhadap
    `frankenphp.h`/`zval.h` + header PHP hasil `php-config --includes`;
    file generated ditangani build script, bukan committed kecuali untuk
    reproducibility (keputusan dicatat di ADR).
15. **Aturan threading Zend**: satu request = satu OS thread, state per-thread
    (TSRM), `frankenphp_new_php_thread()`/destroy hanya dipanggil dari thread
    itu sendiri. Agen WAJIB membaca `docs/internals.md` dan `phpthread.go`
    sebelum menyentuh modul engine.
16. **Tidak ada `async fn` yang melintasi batas FFI.** Fungsi yang memanggil
    Zend harus blocking di `spawn_blocking`/thread khusus. Larangan keras:
    tokio task memegang `zend_executor_globals` lintas `.await`.
17. **Setiap pointer dari C divalidasi non-null** sebelum deref; error C
    (return code `frankenphp_*`) selalu dikonversi ke `Result` Rust — tidak
    ada panic yang melewati batas FFI.

### 2.4 Aturan kualitas

18. Benchmark baseline diambil SEBELUM port (`wrk`/`hey` + harness dari
    `cgi_benchmark_test.go`); setiap PR fase 2–4 melampirkan delta
    throughput/p99 vs baseline.
19. Target coverage per crate baru: ≥ 80% line, 100% untuk jalur `unsafe`.
20. Semua agen menjalankan `cargo miri test` pada crate pure-Rust (tanpa FFI
    nyata) dan melaporkan hasilnya.

---

## 3. TARGET ARSITEKTUR (workspace layout)

```
crates/                           # workspace Cargo (baru, paralel dgn kode Go)
├── frankenphp-sys/               # bindgen + build.rs (cc) → satu-satunya FFI boundary
├── frankenphp-engine/            # PHPMAIN thread, pool worker/reg thread, lifecycle, options
├── frankenphp-http/              # hyper 1.x: HTTP/1.1, H2, H3(quinn), early hints, X-Sendfile
├── frankenphp-config/            # parser kompatibel Caddyfile + JSON + env overlay
├── frankenphp-app/               # bin `frankenphp`: CLI (clap), serve/worker/info/version
├── frankenphp-workers/           # worker mode: request loop, max_requests, backpressure
├── frankenphp-watch/             # hot reload (notify)
├── frankenphp-mercure/           # publisher JWT + hub discovery
├── frankenphp-metrics/           # prometheus client
└── frankenphp-embed/             # embedded app archive (.tar + checksum)
c-sys/                            # frankenphp.c/h, zval.h, emulate_php_cli.* (dipindah, TIDAK ditulis ulang)
tests/oracle/                     # harness yang menjalankan testdata/ PHP lama terhadap binary Rust
go-legacy/                        # (sementara) kode Go existing sbg referensi & oracle
```

Keputusan teknologi kunci (jangan diubah tanpa ADR baru):
runtime **tokio**, HTTP **hyper 1.x + h3**, TLS **rustls**, CLI **clap**,
metrics **prometheus-client**, watch **notify**, logging **tracing**.
Caddy dibuang; fitur Caddy yang dipakai (auto-HTTPS via ACME, static file,
reverse proxy dasar) di-reimplementasi secukupnya di `frankenphp-http/config`.

---

## 4. GAMBARAN TASK (board — detail & status hidup di `TASKS.md`)

ID format: `R<mfase>-<nn>`. Dependensi menentukan urutan eksekusi agen.

### Fase 0 — Spike & De-risking (harus selesai sebelum coding produksi)
- **R0-01** ⚠️ Proof-of-concept: panggil `frankenphp_new_php_thread` +
  `frankenphp_execute_script` dari Rust via bindgen, render 1 halaman PHP.
  *Gate go/no-go seluruh proyek.*
- **R0-02** Audit lengkap permukaan `frankenphp.h` + `zval.h`: daftar simbol
  yang akan di-bind, tandai yang butuh wrapper C tambahan.
- **R0-03** Benchmark baseline Go (throughput, p99, RSS, cold-start) +
  dokumentasi target angka.
- **R0-04** ADR: bindgen-vs-manual, static-vs-dynamic link PHP, pilihan
  library ACME pengganti Caddy.

### Fase 1 — Fondasi
- **R1-01** Inisialisasi Cargo workspace, CI paralel (pipeline Go tidak boleh
  pecah), `rust-toolchain.toml`, `deny.toml` (lisensi), fmt/clippy config.
- **R1-02** Crate `frankenphp-sys`: build.rs (cc+bindgen), matrix multi-PHP
  (8.2/8.3/8.4), cfg platform (linux/macos/windows).
- **R1-03** Skrip oracle: jalankan subset `testdata/*.php` terhadap binary
  referensi Go → simpan golden response.

### Fase 2 — Engine Core (bagian tersulit; pairing FFI-01 + CON-01)
- **R2-01** PHPMAIN thread controller (port `phpmainthread.go`).
- **R2-02** Thread registry + state machine idle/busy/dying (port
  `phpthread.go`, `debugstate.go`; oracle `threadtasks_test.go`).
- **R2-03** Regular (classic) mode request handler (port `cgi.go` —
  `cgi_test.go` 24KB + `cgi_unicode_extra_regression_test.go` = checklist
  variabel CGI literal).
- **R2-04** Worker mode loop (port `worker.go`, `threadworker.go`; oracle
  `maxrequests_regular_test.go`, `persistent_resources_test.go`).
- **R2-05** Options/env mapping (port `options.go`, `env.go`,
  `requestoptions.go`).
- **R2-06** Scaling heuristik (port `scaling.go`) + graceful shutdown/restart
  (signal handling, drain; oracle `signal_linux_test.go`).

### Fase 3 — HTTP Server
- **R3-01** Hyper server + adapter request→CGI context (port `server.go`,
  `context.go`).
- **R3-02** HTTP/2 + HTTP/3 (quinn) + Early Hints (103).
- **R3-03** rustls + ACME (auto-HTTPS) + mTLS (counterpart `mtls-cflags.sh`).
- **R3-04** Request body limits & timeouts (oracle `requestbodylimit_test.go`,
  `requestbodytimeout_test.go`).
- **R3-05** Static files + X-Sendfile + embed archive (port `embed.go`,
  spesifikasi `docs/x-sendfile.md`, `docs/static.md`).

### Fase 4 — Fitur Aplikasi
- **R4-01** Hot reload watcher (port `watcher.go`, `hotreload.go` + internal/watcher).
- **R4-02** Mercure publisher (port `mercure.go`; oracle `mercure_test.go`).
- **R4-03** Metrics endpoint (port `metrics.go`; oracle `metrics_test.go`).
- **R4-04** PHP extension loader & worker extension API (port `ext.go`,
  `workerextension.go`; oracle `workerextension_test.go`).
- **R4-05** phpinfo/status page (port `phpinfo.go`) + CLI subcommands (port
  `cli.go`; oracle `cli_test.go`).

### Fase 5 — Konfigurasi & Kompatibilitas
- **R5-01** Parser Caddyfile-subset + JSON direct-compat
  (`docs/config.md` sebagai spesifikasi).
- **R5-02** Konformance suite: boot sukses dengan semua contoh config dokumentasi.
- **R5-03** Dokumen migrasi user (revisi `docs/migrate.md` edisi Rust).

### Fase 6 — Distribusi
- **R6-01** Build statis musl/macOS/Windows (port `build-static.sh`,
  `vcpkg.json`), Dockerfiles baru, docker-bake matrix.
- **R6-02** Installer (`install.sh`/`install.ps1`), release automation.

### Fase 7 — Hardening & Decommission
- **R7-01** Fuzzing (target httparse-style + mirror `zendalloc_gofuzz.go`),
  `cargo audit`, miri pass penuh.
- **R7-02** Load test soak 24 jam, leak check RSS.
- **R7-03** ⛔ Penghapusan kode Go & toolchain cgo dari produk akhir
  (hanya setelah R7-01/02 hijau + sign-off manusia).

Setiap task di `TASKS.md` wajib berisi: **Depends-on, Files-to-read
(sumber Go), Files-to-create, Definition of Done (daftar test oracle),
Owner-agen.**

---

## 5. KONTRIBUTOR YANG DIBUTUHKAN (spesialisasi)

| ID | Peran | Spesialisasi wajib | Tugas utama | Jumlah |
|----|-------|--------------------|-------------|--------|
| ARC-01 | **System Architect (Rust)** | Arsitektur crate, async runtime internals, API design, penulisan ADR | Menjaga §3, review desain antar-crate, keputusan dependency | 1 |
| FFI-01 | **PHP/Zend Internals Expert** | Zend Engine API, TSRM/GLOBALS, SAPI, `php-config`, zval lifecycle, memory manager | Review R0-01/02, spec wrapper `frankenphp-sys`, audit SAFETY comments | 1 |
| CSHIM-01 | **C Systems Programmer** | C11, build systems (cc/vcpkg), cross-compilation, static linking glibc/musl/libxml/curl | Merawat `c-sys/` & build.rs; memindahkan `frankenphp.c` tanpa rewrite | 1 |
| CON-01 | **Rust Concurrency Engineer** | Tokio internals, spawn_blocking, lock-free structures, loom, diagnosa deadlock/UB | Modul engine thread-pool (R2-*), scaling, graceful shutdown | 1–2 |
| NET-01 | **HTTP Protocol Engineer** | RFC 9110/9113/9114/8297(103), hyper/h3/quinn internals, rustls, ACME | Fase 3, konformance HTTP, perf tuning | 1–2 |
| CFG-01 | **Config/Parser Engineer** | Parsing DSL, semantik Caddyfile, serde, backward-compat testing | Fase 5 (R5-*) | 1 |
| QA-01 | **Test/Oracle Engineer** | Go (membaca & menerjemahkan test suite), proptest, cargo-fuzz, miri, criterion/wrk | R0-03, R1-03, oracle harness, gerbang DoD semua task | 2 |
| REL-01 | **Build/Distribution Engineer** | cargo-dist, musl-cross, Docker multi-stage, GitHub Actions, code signing | Fase 6 | 1 |
| SEC-01 | **Security Reviewer** | Threat modeling server publik, audit CVE, memory safety review, supply-chain (deny.toml) | Review berkala + gate Fase 7 | 1 (part-time) |
| DOC-01 | **Tech Writer (PHP↔Rust)** | Dokumentasi user-facing, migration guide | R5-03, changelog, README | 1 (part-time) |

Catatan untuk orkestrator agen:
- Satu agen AI dapat memerankan beberapa peran HANYA jika tidak melakukan
  self-review/self-approve atas gate QA/SEC milik task sendiri.
- Gate review wajib: **FFI-01** menyetujui semua diff yang menyentuh
  `frankenphp-sys`/`frankenphp-engine`; **QA-01** menyetujui klaim "done";
  **SEC-01** + manusia menyetujui penghapusan Go (R7-03).
- Skill langka (bottleneck Fase 2): pemahaman simultan *Go legacy* +
  *Zend internals* + *unsafe Rust*. Rencanakan pairing FFI-01 + CON-01 pada
  task yang sama.

---

## 6. ALUR KERJA PER-TASK (SOP agen)

```
baca TASKS.md → klaim task → baca Files-to-read (Go source + test + docs/internals.md)
→ tulis test Rust translasi (merah) → implementasi → fmt/clippy/test/miri
→ benchmark (jika diminta DoD) → update TASKS.md + ADR/catatan → lampirkan bukti → PR
```

Definisi "Done" universal: ✅ build ✅ `clippy -D warnings` ✅ test unit
✅ oracle test terkait hijau ✅ SAFETY comments lengkap ✅ dokumentasi crate
diperbarui ✅ tidak menyentuh file di luar scope task.

---

## 7. ESKALASI & HAL-HAL YANG BAHAYA

Eskalasi ke manusia segera jika menemukan:
- Segfault/UB saat eksperimen FFI (catat reproduksi minimal + core dump).
- Kebutuhan mengubah `frankenphp.c` di luar penambahan wrapper kecil.
- Konflik lisensi dependency atau kebutuhan crate non-standard.
- Test oracle Go yang bergantung pada quirk tak terdokumentasi (mis. ordering
  header, edge-case unicode di `cgi_unicode_extra_regression_test.go`)
  → default: replikasi quirk; perbaikan = breaking-change, butuh keputusan
  manusia.
- Tekanan untuk menghapus kode Go lebih cepat dari gerbang Fase 7.

Daftar hitam perilaku agen: menulis binding Zend manual secara massal,
mengubah test agar sesuai implementasi, `block_on` di dalam thread PHP,
mengubah semantik `max_requests`, melemahkan lint gate tanpa jejak ADR.

---

*File pendamping (direktori sama): `TASKS.md` (board hidup),
`DEPENDENCIES.md` (registry crate), `adr/` (keputusan arsitektur).
Ketiganya wajib dibaca sebelum sesi kerja pertama setiap agen.*
