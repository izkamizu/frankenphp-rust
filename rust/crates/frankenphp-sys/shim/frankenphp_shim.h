#ifndef FRANKENPHP_SHIM_H
#define FRANKENPHP_SHIM_H
#include <stddef.h>
int fp_startup(void);
void fp_shutdown(void);
int fp_handle_request(const char *script_path, char **out_buf, size_t *out_len);
const char *fp_version(void);
void fp_free(void *p);
#endif
