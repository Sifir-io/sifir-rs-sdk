#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct {
  const uint8_t *bytes;
  uintptr_t len;
} RustByteSlice;

RustByteSlice send_request(void);
