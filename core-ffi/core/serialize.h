#ifndef KADEMLIA_SERIALIZE_H
#define KADEMLIA_SERIALIZE_H

#include <stdint.h>
#include <stddef.h>

#define KADEMLIA_MAGIC_0 0x4B
#define KADEMLIA_MAGIC_1 0x44
#define ID_SIZE 20
#define MAX_VALUE_SIZE 65536
#define MAX_CONTACTS 20

typedef enum {
    MSG_PING = 1,
    MSG_PONG = 2,
    MSG_STORE = 3,
    MSG_STORE_ACK = 4,
    MSG_FIND_NODE = 5,
    MSG_FIND_NODE_RESP = 6,
    MSG_FIND_VALUE = 7,
    MSG_FIND_VALUE_RESP = 8
} MessageType;

typedef struct {
    uint8_t id[ID_SIZE];
    uint8_t ip_type;    /* 4 for IPv4, 6 for IPv6 */
    uint8_t ip[16];     /* 4 bytes used for IPv4, 16 for IPv6 */
    uint16_t port;
} SerializedContact;

typedef struct {
    uint8_t type;
    uint64_t request_id;
    uint8_t sender_id[ID_SIZE];

    union {
        /* STORE (type == MSG_STORE) */
        struct {
            uint8_t key[ID_SIZE];
            uint32_t value_len;
            uint8_t value[MAX_VALUE_SIZE];
        } store;

        /* STORE_ACK (type == MSG_STORE_ACK) */
        struct {
            uint8_t status;
        } store_ack;

        /* FIND_NODE (type == MSG_FIND_NODE) */
        struct {
            uint8_t target_id[ID_SIZE];
        } find_node;

        /* FIND_NODE_RESP (type == MSG_FIND_NODE_RESP) */
        struct {
            uint8_t count;
            SerializedContact contacts[MAX_CONTACTS];
        } find_node_resp;

        /* FIND_VALUE (type == MSG_FIND_VALUE) */
        struct {
            uint8_t key[ID_SIZE];
        } find_value;

        /* FIND_VALUE_RESP (type == MSG_FIND_VALUE_RESP) */
        struct {
            uint8_t has_value;
            uint32_t value_len;
            uint8_t value[MAX_VALUE_SIZE];
            uint8_t count;
            SerializedContact contacts[MAX_CONTACTS];
        } find_value_resp;
    } payload;
} Message;

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Serializes Message struct into out_buf.
 * Returns the number of bytes written, or negative error code on failure.
 */
int encode_message(const Message* msg, uint8_t* out_buf, size_t buf_len);

/*
 * Deserializes buf into Message struct.
 * Returns 0 on success, or negative error code on failure.
 */
int decode_message(const uint8_t* buf, size_t len, Message* out_msg);

#ifdef __cplusplus
}
#endif

#endif /* KADEMLIA_SERIALIZE_H */
