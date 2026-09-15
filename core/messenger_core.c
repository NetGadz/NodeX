#include "messenger_core.h"
#include <stdio.h>
#include <string.h>

static const char HEX_CHARS[] = "0123456789abcdef";

void c_node_id_to_hex(const uint8_t* id_bytes, char* out_hex_buf) {
    if (!id_bytes || !out_hex_buf) return;
    for (size_t i = 0; i < C_ID_SIZE; i++) {
        out_hex_buf[i * 2]     = HEX_CHARS[(id_bytes[i] >> 4) & 0x0F];
        out_hex_buf[i * 2 + 1] = HEX_CHARS[id_bytes[i] & 0x0F];
    }
    out_hex_buf[C_ID_SIZE * 2] = '\0';
}

int c_messenger_derive_mailbox_key(
    const uint8_t* user_id_bytes,
    uint64_t timestamp,
    char* out_key_buf,
    size_t buf_len
) {
    if (!user_id_bytes || !out_key_buf || buf_len < 64) {
        return -1;
    }

    char hex_id[C_ID_SIZE * 2 + 1];
    c_node_id_to_hex(user_id_bytes, hex_id);

    uint64_t day_epoch = timestamp / 86400;

    int written = snprintf(out_key_buf, buf_len, "mailbox_%s_%llu", hex_id, (unsigned long long)day_epoch);
    if (written < 0 || (size_t)written >= buf_len) {
        return -2;
    }

    return written;
}

int c_messenger_format_presence_payload(
    const uint8_t* user_id_bytes,
    const char* socket_addr_str,
    uint64_t timestamp,
    uint8_t* out_buf,
    size_t buf_len
) {
    if (!user_id_bytes || !socket_addr_str || !out_buf) {
        return -1;
    }

    size_t addr_len = strlen(socket_addr_str);
    size_t total_needed = C_ID_SIZE + addr_len + sizeof(uint64_t);

    if (buf_len < total_needed) {
        return -2;
    }

    memcpy(out_buf, user_id_bytes, C_ID_SIZE);
    memcpy(out_buf + C_ID_SIZE, socket_addr_str, addr_len);

    uint64_t time_be = 0;
    for (int i = 0; i < 8; i++) {
        ((uint8_t*)&time_be)[i] = (uint8_t)(timestamp >> (56 - i * 8));
    }

    memcpy(out_buf + C_ID_SIZE + addr_len, &time_be, sizeof(uint64_t));

    return (int)total_needed;
}

int c_messenger_create_header(
    const uint8_t* sender_id,
    const uint8_t* recipient_id,
    uint64_t timestamp,
    uint32_t payload_len,
    CMessengerHeader* out_header
) {
    if (!sender_id || !recipient_id || !out_header) {
        return -1;
    }

    out_header->magic[0] = 'V';
    out_header->magic[1] = 'X';
    out_header->version = 1;
    out_header->flags = 0;
    memcpy(out_header->sender_id, sender_id, C_ID_SIZE);
    memcpy(out_header->recipient_id, recipient_id, C_ID_SIZE);
    out_header->timestamp = timestamp;
    out_header->payload_len = payload_len;

    return 0;
}

int c_messenger_validate_header(const CMessengerHeader* header) {
    if (!header) return -1;
    if (header->magic[0] != 'V' || header->magic[1] != 'X') return -2;
    if (header->version != 1) return -3;
    return 0;
}

uint32_t c_messenger_calculate_checksum(const uint8_t* data, size_t len) {
    if (!data || len == 0) return 0;
    uint32_t crc = 0xFFFFFFFF;
    for (size_t i = 0; i < len; i++) {
        crc ^= data[i];
        for (int j = 0; j < 8; j++) {
            crc = (crc >> 1) ^ (0xEDB88320 & (-(crc & 1)));
        }
    }
    return ~crc;
}
