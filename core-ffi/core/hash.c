#include "hash.h"
#include <string.h>

static inline int count_leading_zeros_u8(uint8_t b) {
    if (b == 0) return 8;
    int n = 0;
    if ((b & 0xF0) == 0) { n += 4; b <<= 4; }
    if ((b & 0xC0) == 0) { n += 2; b <<= 2; }
    if ((b & 0x80) == 0) { n += 1; }
    return n;
}

uint32_t c_shared_prefix_bits(const uint8_t* id1, const uint8_t* id2, size_t len) {
    if (!id1 || !id2) {
        return 0;
    }
    uint32_t total = 0;
    for (size_t i = 0; i < len; ++i) {
        uint8_t diff = id1[i] ^ id2[i];
        if (diff == 0) {
            total += 8;
        } else {
            total += (uint32_t)count_leading_zeros_u8(diff);
            break;
        }
    }
    return total;
}

/* SHA-1 Implementation */
typedef struct {
    uint32_t state[5];
    uint32_t count[2];
    uint8_t buffer[64];
} SHA1_CTX;

#define SHA1_ROL(value, bits) (((value) << (bits)) | ((value) >> (32 - (bits))))

static void sha1_transform(uint32_t state[5], const uint8_t buffer[64]) {
    uint32_t a = state[0], b = state[1], c = state[2], d = state[3], e = state[4];
    uint32_t w[80];

    for (int i = 0; i < 16; ++i) {
        w[i] = ((uint32_t)buffer[i * 4] << 24) |
               ((uint32_t)buffer[i * 4 + 1] << 16) |
               ((uint32_t)buffer[i * 4 + 2] << 8) |
               ((uint32_t)buffer[i * 4 + 3]);
    }
    for (int i = 16; i < 80; ++i) {
        w[i] = SHA1_ROL(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
    }

    for (int i = 0; i < 80; ++i) {
        uint32_t f, k;
        if (i < 20) {
            f = (b & c) | ((~b) & d);
            k = 0x5A827999;
        } else if (i < 40) {
            f = b ^ c ^ d;
            k = 0x6ED9EBA1;
        } else if (i < 60) {
            f = (b & c) | (b & d) | (c & d);
            k = 0x8F1BBCDC;
        } else {
            f = b ^ c ^ d;
            k = 0xCA62C1D6;
        }

        uint32_t temp = SHA1_ROL(a, 5) + f + e + k + w[i];
        e = d;
        d = c;
        c = SHA1_ROL(b, 30);
        b = a;
        a = temp;
    }

    state[0] += a;
    state[1] += b;
    state[2] += c;
    state[3] += d;
    state[4] += e;
}

static void sha1_init(SHA1_CTX* context) {
    context->state[0] = 0x67452301;
    context->state[1] = 0xEFCDAB89;
    context->state[2] = 0x98BADCFE;
    context->state[3] = 0x10325476;
    context->state[4] = 0xC3D2E1F0;
    context->count[0] = 0;
    context->count[1] = 0;
}

static void sha1_update(SHA1_CTX* context, const uint8_t* data, size_t len) {
    size_t i = 0;
    size_t j = (context->count[0] >> 3) & 63;
    if ((context->count[0] += ((uint32_t)len << 3)) < ((uint32_t)len << 3)) {
        context->count[1]++;
    }
    context->count[1] += (uint32_t)(len >> 29);

    if ((j + len) > 63) {
        memcpy(&context->buffer[j], data, (i = 64 - j));
        sha1_transform(context->state, context->buffer);
        for (; i + 63 < len; i += 64) {
            sha1_transform(context->state, &data[i]);
        }
        j = 0;
    }
    memcpy(&context->buffer[j], &data[i], len - i);
}

static void sha1_final(uint8_t digest[20], SHA1_CTX* context) {
    uint8_t finalcount[8];
    for (int i = 0; i < 8; ++i) {
        finalcount[i] = (uint8_t)((context->count[(i >= 4 ? 0 : 1)] >> ((3 - (i & 3)) * 8)) & 255);
    }
    sha1_update(context, (const uint8_t*)"\200", 1);
    while ((context->count[0] & 504) != 448) {
        sha1_update(context, (const uint8_t*)"\0", 1);
    }
    sha1_update(context, finalcount, 8);
    for (int i = 0; i < 20; ++i) {
        digest[i] = (uint8_t)((context->state[i >> 2] >> ((3 - (i & 3)) * 8)) & 255);
    }
}

void hash_node_id(const uint8_t* input, size_t len, uint8_t out_id[20]) {
    SHA1_CTX ctx;
    sha1_init(&ctx);
    if (input && len > 0) {
        sha1_update(&ctx, input, len);
    }
    sha1_final(out_id, &ctx);
}
