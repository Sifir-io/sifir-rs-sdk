#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Since the FFI simply starts and shutdowns the daemon we use an
 * Opaque pointer here to pass across the FFI
 */
typedef struct {
  OwnedTorService *service;
} OwnedTorBoxed;

OwnedTorBoxed *get_owned_TorService(const char *data_dir, uint16_t socks_port);

/**
 *# Safety
 * Destroy and release ownedTorBox which will shut down owned connection and shutdown daemon
 */
void shutdown_owned_TorService(OwnedTorBoxed *owned_client);
