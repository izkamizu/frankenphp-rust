# TASKS — Board Migrasi Rust (dokumen hidup)

> Format tiap task lihat `AGENTS.md` §4. Status: `TODO | IN-PROGRESS @agen | REVIEW | DONE | BLOCKED`.
> Aturan: klaim dengan mengubah status + agen-id dalam commit terpisah sebelum mulai kerja.

| ID | Task | Depends-on | Role | Status | DoD ringkas |
|----|------|-----------|------|--------|-------------|
| R0-01 | Spike FFI: eksekusi 1 script PHP dari Rust | — | FFI-01, CON-01 | TODO | PoC binary render `testdata/index.php`, no segfault di 100 iterasi |
| R0-02 | Audit simbol `frankenphp.h`/`zval.h` | — | FFI-01 | TODO | Daftar simbol + wrapper C yang dibutuhkan |
| R0-03 | Benchmark baseline Go | — | QA-01 | TODO | Angka RPS/p99/RSS/cold-start terdokumentasi |
| R0-04 | ADR: bindgen, linking, ACME | R0-01 | ARC-01 | TODO | adr/001–003 merged |
| R1-01 | Cargo workspace + CI paralel | R0-04 | ARC-01, REL-01 | TODO | `cargo build` hijau di CI, pipeline Go utuh |
| R1-02 | Crate `frankenphp-sys` | R0-02, R1-01 | FFI-01, CSHIM-01 | TODO | Binding generated utk PHP 8.2–8.4, 3 OS |
| R1-03 | Oracle harness golden-response | R0-03 | QA-01 | TODO | Subset testdata → snapshot JSON otomatis |
| R2-01 | PHPMAIN controller | R1-02 | FFI-01, CON-01 | TODO | Oracle `phpmainthread_test.go` translasi hijau |
| R2-02 | Thread registry + state machine | R2-01 | CON-01 | TODO | loom pass + oracle threadtasks |
| R2-03 | Classic mode / CGI vars | R2-02 | FFI-01, QA-01 | TODO | cgi_test.rs ≡ cgi_test.go lulus |
| R2-04 | Worker mode loop | R2-02 | CON-01 | TODO | max_requests & persistent resources hijau |
| R2-05 | Options/env mapping | R2-01 | CFG-01 | TODO | options_test.rs hijau |
| R2-06 | Scaling + graceful shutdown | R2-04 | CON-01 | TODO | scaling_test + signal_test hijau |
| R3-01 | Hyper server + CGI adapter | R2-03 | NET-01 | TODO | server_test.rs hijau |
| R3-02 | H2/H3 + Early Hints | R3-01 | NET-01 | TODO | Test 103 via h2spec/quic wpt subset |
| R3-03 | rustls + ACME + mTLS | R3-01 | NET-01 | TODO | Certbot-less e2e HTTPS lulus |
| R3-04 | Body limits/timeouts | R3-01 | NET-01 | TODO | requestbody* tests hijau |
| R3-05 | Static + X-Sendfile + embed | R3-01 | NET-01, CFG-01 | TODO | embed_test + x-sendfile hijau |
| R4-01 | Hot reload watcher | R2-04 | CON-01 | TODO | watcher/hotreload tests hijau |
| R4-02 | Mercure publisher | R3-01 | NET-01 | TODO | mercure_test.rs hijau |
| R4-03 | Metrics endpoint | R3-01 | QA-01 | TODO | metrics_test.rs hijau |
| R4-04 | Extension loader + worker ext API | R2-04 | FFI-01, CSHIM-01 | TODO | workerextension tests hijau |
| R4-05 | phpinfo/status + CLI | R3-01 | CFG-01 | TODO | cli_test.rs hijau |
| R5-01 | Caddyfile-subset + JSON parser | R3-01 | CFG-01 | TODO | Semua contoh docs/config.md parse |
| R5-02 | Konformance config suite | R5-01 | QA-01 | TODO | Boot matrix config lulus |
| R5-03 | Dokumen migrasi user | R5-02 | DOC-01 | TODO | Review manusia |
| R6-01 | Build statis multi-platform | R5-02 | CSHIM-01, REL-01 | TODO | Artifact musl/mac/win boot di smoke test |
| R6-02 | Installer + release automation | R6-01 | REL-01 | TODO | Dry-run release hijau |
| R7-01 | Fuzz + audit + miri penuh | R6-01 | SEC-01, QA-01 | TODO | 0 high CVE, 24h fuzz bersih |
| R7-02 | Soak 24 jam + leak check | R7-01 | QA-01 | TODO | RSS stabil < 5% drift |
| R7-03 | ⛔ Hapus kode Go (sign-off manusia) | R7-02 | ARC-01 + manusia | TODO | Parity report 100%, persetujuan tertulis |
