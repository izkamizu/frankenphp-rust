/*
 * frankenphp_shim.c — thin, stable C shim between Rust and the PHP embed SAPI.
 * Batch 1 scope: engine lifecycle + single-script execution with in-memory
 * output capture (classic-mode smoke test). All Zend/PHP internals stay
 * behind this file (ADR-0001). No Go, no cgo.
 */

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "SAPI.h"
#include "php.h"
#include "php_main.h"
#include "zend_compile.h"

#include "php_embed.h"

static int g_started = 0;

/* ---- request-scoped output capture ---------------------------------------
 * The SAPI `ub_write` hook is the single funnel for all script output once
 * php_output flushes it. We swap it per-request to collect into a growable
 * buffer (same technique FrankenPHP uses in frankenphp.c, self-contained). */

typedef struct { char *buf; size_t len, cap; } fp_sink_t;
static fp_sink_t g_sink;

static size_t fp_capture_write(const char *str, size_t str_length) {
    if (g_sink.len + str_length > g_sink.cap) {
        size_t ncap = g_sink.cap ? g_sink.cap : 8192;
        while (ncap < g_sink.len + str_length) ncap *= 2;
        char *nb = realloc(g_sink.buf, ncap);
        if (!nb) return 0; /* drop on OOM like embed default */
        g_sink.buf = nb;
        g_sink.cap = ncap;
    }
    memcpy(g_sink.buf + g_sink.len, str, str_length);
    g_sink.len += str_length;
    return str_length;
}

/* ---- lifecycle ------------------------------------------------------------ */

int fp_startup(void) {
    if (g_started) return 0;
    sapi_startup(&php_embed_module);
    if (php_embed_init(0, NULL) == FAILURE) {
        sapi_shutdown();
        return -1;
    }
    g_started = 1;
    return 0;
}

void fp_shutdown(void) {
    if (!g_started) return;
    php_embed_shutdown();
    sapi_shutdown();
    g_started = 0;
}

/* ---- request execution -----------------------------------------------------
 * One full request cycle for `script_path`. On success returns 0 and sets
 * *out_buf (malloc'ed, free with fp_free) / *out_len. Negative = error code.
 */
int fp_handle_request(const char *script_path, char **out_buf, size_t *out_len) {
    *out_buf = NULL;
    *out_len = 0;
    if (!g_started || !script_path) return -1;

    if (php_request_startup() == FAILURE) return -2;

    /* install capture hook for this request */
    size_t (*saved_ub_write)(const char *, size_t) = sapi_module.ub_write;
    sapi_module.ub_write = fp_capture_write;
    g_sink.len = 0;

    zend_file_handle fh;
    memset(&fh, 0, sizeof(fh));
    fh.type = ZEND_HANDLE_FILENAME;
    fh.filename = zend_string_init(script_path, strlen(script_path), 0);
    fh.opened_path = NULL;
    fh.primary_script = 1;

    int rc = 0;
    zend_try {
        if (zend_execute_scripts(ZEND_REQUIRE, NULL, 1, &fh) == FAILURE) {
            rc = -3;
        }
    } zend_end_try();

    /* If the file was never opened (e.g. parse error before open), release
     * the filename we allocated. When opened successfully, primary_script
     * bookkeeping owns it — matches upstream embed/cli handling. */
    if (!fh.opened_path) {
        zend_string_release(fh.filename);
    }

    /* flush remaining buffered output through our hook, then restore */
    php_output_deactivate();
    sapi_module.ub_write = saved_ub_write;

    php_request_shutdown(NULL);

    if (rc != 0) {
        free(g_sink.buf);
        g_sink.buf = NULL; g_sink.cap = 0; g_sink.len = 0;
        return rc;
    }

    *out_buf = g_sink.buf;
    *out_len = g_sink.len;
    g_sink.buf = NULL; g_sink.cap = 0; g_sink.len = 0;
    return 0;
}

const char *fp_version(void) { return PHP_VERSION; }

void fp_free(void *p) { free(p); }
