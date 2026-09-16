#ifndef KADEMLIA_MESSENGER_CORE_H
#define KADEMLIA_MESSENGER_CORE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define C_ID_SIZE 20
#define C_KEY_SIZE 32
#define C_SIG_SIZE 64
#define C_MAX_MAILBOX_KEY_LEN 128

typedef struct {
    uint8_t magic[2];      /* 'V', 'X' */
    uint8_t version;       /* 1 */
    uint8_t flags;         /* 0 = direct, 1 = offline relay */
    uint8_t sender_id[C_ID_SIZE];
    uint8_t recipient_id[C_ID_SIZE];
    uint64_t timestamp;
    uint32_t payload_len;
} CMessengerHeader;

/*
 * Derives a deterministic Kademlia Mailbox key for offline store-and-forward relay.
 * Format: "mailbox_<hex_user_id>_<day_epoch>"
 * Returns number of bytes written to out_key, or negative on error.
 */
int c_messenger_derive_mailbox_key(
    const uint8_t* user_id_bytes,
    uint64_t timestamp,
    char* out_key_buf,
    size_t buf_len
);

/*
 * Formats a binary presence payload for Ed25519 signature.
 * Returns number of bytes written to out_buf, or negative on error.
 */
int c_messenger_format_presence_payload(
    const uint8_t* user_id_bytes,
    const char* socket_addr_str,
    uint64_t timestamp,
    uint8_t* out_buf,
    size_t buf_len
);

/*
 * Creates and validates a binary CMessengerHeader struct in pure C.
 */
int c_messenger_create_header(
    const uint8_t* sender_id,
    const uint8_t* recipient_id,
    uint64_t timestamp,
    uint32_t payload_len,
    CMessengerHeader* out_header
);

int c_messenger_validate_header(const CMessengerHeader* header);

/*
 * Calculates simple checksum digest for packet validation.
 */
uint32_t c_messenger_calculate_checksum(const uint8_t* data, size_t len);

/*
 * Converts a 20-byte binary NodeId to a 40-character hex string.
 */
void c_node_id_to_hex(const uint8_t* id_bytes, char* out_hex_buf);

#ifdef __cplusplus
}
#endif

#endif /* KADEMLIA_MESSENGER_CORE_H */
