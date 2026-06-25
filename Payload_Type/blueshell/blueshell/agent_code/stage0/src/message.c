#include "stage0.h"

static void s0_copy(void *dst, const void *src, uint32_t n) {
    uint8_t *d = dst; const uint8_t *s = src;
    while (n--) *d++ = *s++;
}

int s0_frame_pack(uint8_t kind, uint16_t flags, uint32_t stream_id,
                  const void *payload, uint32_t length, s0_buffer *out) {
    s0_frame_header h;
    uint32_t total = (uint32_t)sizeof(h) + length;
    if (!out || length > S0_MAX_FRAME - sizeof(h) ||
        !s0_buffer_reserve(out, total)) return 0;
    h.magic = S0_MAGIC; h.version = S0_VERSION; h.kind = kind;
    h.flags = flags; h.stream_id = stream_id; h.length = length;
    h.checksum = s0_checksum(payload, length);
    s0_copy(out->data, &h, sizeof(h));
    if (length) s0_copy(out->data + sizeof(h), payload, length);
    out->length = total;
    return 1;
}

int s0_frame_unpack(const uint8_t *data, uint32_t length,
                    s0_frame_header *header, const uint8_t **payload) {
    if (!data || !header || !payload || length < sizeof(*header)) return 0;
    *header = *(const s0_frame_header *)data;
    if (header->magic != S0_MAGIC || header->version != S0_VERSION ||
        header->length > S0_MAX_FRAME ||
        sizeof(*header) + header->length > length) return 0;
    *payload = data + sizeof(*header);
    return s0_checksum(*payload, header->length) == header->checksum;
}

int s0_proxy_pack(uint8_t kind, uint32_t server_id, uint32_t port,
                  int exit_flag, const void *data, uint32_t length,
                  s0_buffer *out) {
    s0_proxy_frame p;
    s0_buffer inner = {0};
    if (kind != S0_MSG_SOCKS && kind != S0_MSG_RPFWD) return 0;
    if (!s0_buffer_reserve(&inner, sizeof(p) + length)) return 0;
    p.server_id = server_id; p.port = port; p.length = length;
    p.exit = exit_flag ? 1u : 0u;
    p.reserved[0] = p.reserved[1] = p.reserved[2] = 0;
    s0_copy(inner.data, &p, sizeof(p));
    if (length) s0_copy(inner.data + sizeof(p), data, length);
    inner.length = sizeof(p) + length;
    if (!s0_frame_pack(kind, 0, server_id, inner.data, inner.length, out)) {
        s0_buffer_free(&inner); return 0;
    }
    s0_buffer_free(&inner);
    return 1;
}

