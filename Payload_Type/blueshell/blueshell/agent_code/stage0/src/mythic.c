#include "stage0.h"
#include <bcrypt.h>
#include <wincrypt.h>

static int s0_aes(const uint8_t key[32], const uint8_t iv[16],
                  const uint8_t *input, uint32_t input_len,
                  int encrypt, s0_buffer *output) {
    BCRYPT_ALG_HANDLE alg = 0;
    BCRYPT_KEY_HANDLE kh = 0;
    DWORD object_len = 0, cb = 0, wanted = 0;
    uint8_t *object = 0;
    uint8_t iv_copy[16];
    NTSTATUS status;
    int ok = 0;
    CopyMemory(iv_copy, iv, sizeof(iv_copy));
    if (BCryptOpenAlgorithmProvider(&alg, BCRYPT_AES_ALGORITHM, 0, 0) < 0 ||
        BCryptSetProperty(alg, BCRYPT_CHAINING_MODE,
                          (PUCHAR)BCRYPT_CHAIN_MODE_CBC,
                          sizeof(BCRYPT_CHAIN_MODE_CBC), 0) < 0 ||
        BCryptGetProperty(alg, BCRYPT_OBJECT_LENGTH, (PUCHAR)&object_len,
                          sizeof(object_len), &cb, 0) < 0)
        goto done;
    object = HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, object_len);
    if (!object ||
        BCryptGenerateSymmetricKey(alg, &kh, object, object_len,
                                   (PUCHAR)key, 32, 0) < 0)
        goto done;
    status = encrypt
        ? BCryptEncrypt(kh, (PUCHAR)input, input_len, 0, iv_copy, 16,
                        0, 0, &wanted, BCRYPT_BLOCK_PADDING)
        : BCryptDecrypt(kh, (PUCHAR)input, input_len, 0, iv_copy, 16,
                        0, 0, &wanted, BCRYPT_BLOCK_PADDING);
    if (status < 0 || !s0_buffer_reserve(output, wanted)) goto done;
    CopyMemory(iv_copy, iv, sizeof(iv_copy));
    status = encrypt
        ? BCryptEncrypt(kh, (PUCHAR)input, input_len, 0, iv_copy, 16,
                        output->data, output->capacity, &wanted,
                        BCRYPT_BLOCK_PADDING)
        : BCryptDecrypt(kh, (PUCHAR)input, input_len, 0, iv_copy, 16,
                        output->data, output->capacity, &wanted,
                        BCRYPT_BLOCK_PADDING);
    if (status >= 0) { output->length = wanted; ok = 1; }
done:
    if (kh) BCryptDestroyKey(kh);
    if (alg) BCryptCloseAlgorithmProvider(alg, 0);
    if (object) { SecureZeroMemory(object, object_len); HeapFree(GetProcessHeap(), 0, object); }
    return ok;
}

static int s0_hmac(const uint8_t key[32], const uint8_t *data,
                   uint32_t length, uint8_t digest[32]) {
    BCRYPT_ALG_HANDLE alg = 0;
    BCRYPT_HASH_HANDLE hash = 0;
    DWORD object_len = 0, cb = 0;
    uint8_t *object = 0;
    int ok = 0;
    if (BCryptOpenAlgorithmProvider(&alg, BCRYPT_SHA256_ALGORITHM, 0,
                                    BCRYPT_ALG_HANDLE_HMAC_FLAG) >= 0 &&
        BCryptGetProperty(alg, BCRYPT_OBJECT_LENGTH, (PUCHAR)&object_len,
                          sizeof(object_len), &cb, 0) >= 0 &&
        (object = HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY,
                            object_len)) != 0 &&
        BCryptCreateHash(alg, &hash, object, object_len,
                         (PUCHAR)key, 32, 0) >= 0 &&
        BCryptHashData(hash, (PUCHAR)data, length, 0) >= 0 &&
        BCryptFinishHash(hash, digest, 32, 0) >= 0)
        ok = 1;
    if (hash) BCryptDestroyHash(hash);
    if (alg) BCryptCloseAlgorithmProvider(alg, 0);
    if (object) {
        SecureZeroMemory(object, object_len);
        HeapFree(GetProcessHeap(), 0, object);
    }
    return ok;
}

int s0_mythic_encode(const char *uuid, const uint8_t key[32],
                     const void *json, uint32_t json_len, s0_buffer *out) {
    s0_buffer cipher = {0}, raw = {0};
    uint8_t iv[16], digest[32];
    DWORD encoded_len = 0;
    int ok = 0;
    if (!uuid || lstrlenA(uuid) != 36 ||
        BCryptGenRandom(0, iv, sizeof(iv), BCRYPT_USE_SYSTEM_PREFERRED_RNG) < 0 ||
        !s0_aes(key, iv, json, json_len, 1, &cipher) ||
        !s0_buffer_reserve(&raw, 36 + 16 + cipher.length + 32))
        goto done;
    CopyMemory(raw.data, uuid, 36);
    CopyMemory(raw.data + 36, iv, 16);
    CopyMemory(raw.data + 52, cipher.data, cipher.length);
    raw.length = 52 + cipher.length;
    if (!s0_hmac(key, raw.data + 36, raw.length - 36, digest)) goto done;
    CopyMemory(raw.data + raw.length, digest, 32);
    raw.length += 32;
    if (!CryptBinaryToStringA(raw.data, raw.length,
                              CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF,
                              0, &encoded_len) ||
        !s0_buffer_reserve(out, encoded_len))
        goto done;
    if (CryptBinaryToStringA(raw.data, raw.length,
                             CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF,
                             (LPSTR)out->data, &encoded_len)) {
        out->length = encoded_len - 1;
        ok = 1;
    }
done:
    s0_buffer_free(&cipher);
    s0_buffer_free(&raw);
    return ok;
}

int s0_mythic_decode(const uint8_t key[32], const void *encoded,
                     uint32_t encoded_len, char uuid[37], s0_buffer *json) {
    s0_buffer raw = {0};
    DWORD raw_len = 0;
    uint8_t digest[32], diff = 0;
    uint32_t i, signed_len;
    int ok = 0;
    if (!CryptStringToBinaryA(encoded, encoded_len, CRYPT_STRING_BASE64,
                              0, &raw_len, 0, 0) ||
        raw_len < 36 + 16 + 16 + 32 ||
        !s0_buffer_reserve(&raw, raw_len) ||
        !CryptStringToBinaryA(encoded, encoded_len, CRYPT_STRING_BASE64,
                              raw.data, &raw_len, 0, 0))
        goto done;
    raw.length = raw_len;
    signed_len = raw_len - 36 - 32;
    if (!s0_hmac(key, raw.data + 36, signed_len, digest)) goto done;
    for (i = 0; i < 32; ++i)
        diff |= digest[i] ^ raw.data[raw_len - 32 + i];
    if (diff) goto done;
    CopyMemory(uuid, raw.data, 36);
    uuid[36] = 0;
    ok = s0_aes(key, raw.data + 36, raw.data + 52,
                signed_len - 16, 0, json);
done:
    s0_buffer_free(&raw);
    return ok;
}
