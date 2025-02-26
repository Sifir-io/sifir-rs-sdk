#ifndef TOR_FFI_H
#define TOR_FFI_H

#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

namespace tor {

struct TOR_HiddenServiceResponse {
  bool is_success;
  char *onion_address;
  char *control;
};

struct TOR_StartTorResponse {
  bool is_success;
  char *onion_address;
  char *control;
  char *error_message;
};

extern "C" {

bool initialize_tor_library();

bool init_tor_service(unsigned short socks_port, const char *data_dir, unsigned long timeout_ms);

TOR_HiddenServiceResponse create_hidden_service(unsigned short port,
                                                unsigned short target_port,
                                                const unsigned char *key_data,
                                                bool has_key);

TOR_StartTorResponse start_tor_if_not_running(const char *data_dir,
                                              const unsigned char *key_data,
                                              bool has_key,
                                              unsigned short socks_port,
                                              unsigned short target_port,
                                              unsigned long timeout_ms);

int get_service_status();

bool delete_hidden_service(const char *address);

bool shutdown_service();

void free_string(char *s);

} // extern "C"

} // namespace tor

#endif // TOR_FFI_H
