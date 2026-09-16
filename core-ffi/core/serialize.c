#include "serialize.h"
#include <string.h>

static inline void write_u16_be(uint8_t* p, uint16_t v) {
    p[0] = (uint8_t)(v >> 8);
    p[1] = (uint8_t)(v & 0xFF);
}

static inline uint16_t read_u16_be(const uint8_t* p) {
    return ((uint16_t)p[0] << 8) | (uint16_t)p[1];
}

static inline void write_u32_be(uint8_t* p, uint32_t v) {
    p[0] = (uint8_t)(v >> 24);
    p[1] = (uint8_t)(v >> 16);
    p[2] = (uint8_t)(v >> 8);
    p[3] = (uint8_t)(v & 0xFF);
}

static inline uint32_t read_u32_be(const uint8_t* p) {
    return ((uint32_t)p[0] << 24) |
           ((uint32_t)p[1] << 16) |
           ((uint32_t)p[2] << 8)  |
           ((uint32_t)p[3]);
}

static inline void write_u64_be(uint8_t* p, uint64_t v) {
    p[0] = (uint8_t)(v >> 56);
    p[1] = (uint8_t)(v >> 48);
    p[2] = (uint8_t)(v >> 40);
    p[3] = (uint8_t)(v >> 32);
    p[4] = (uint8_t)(v >> 24);
    p[5] = (uint8_t)(v >> 16);
    p[6] = (uint8_t)(v >> 8);
    p[7] = (uint8_t)(v & 0xFF);
}

static inline uint64_t read_u64_be(const uint8_t* p) {
    return ((uint64_t)p[0] << 56) |
           ((uint64_t)p[1] << 48) |
           ((uint64_t)p[2] << 40) |
           ((uint64_t)p[3] << 32) |
           ((uint64_t)p[4] << 24) |
           ((uint64_t)p[5] << 16) |
           ((uint64_t)p[6] << 8)  |
           ((uint64_t)p[7]);
}

static int encode_contact(const SerializedContact* c, uint8_t* buf, size_t max_len, size_t* out_written) {
    size_t ip_len = (c->ip_type == 6) ? 16 : 4;
    size_t needed = ID_SIZE + 1 + ip_len + 2;
    if (max_len < needed) return -1;

    size_t pos = 0;
    memcpy(&buf[pos], c->id, ID_SIZE);
    pos += ID_SIZE;
    buf[pos++] = c->ip_type;
    memcpy(&buf[pos], c->ip, ip_len);
    pos += ip_len;
    write_u16_be(&buf[pos], c->port);
    pos += 2;

    *out_written = pos;
    return 0;
}

static int decode_contact(const uint8_t* buf, size_t len, SerializedContact* c, size_t* out_read) {
    if (len < ID_SIZE + 1) return -1;
    size_t pos = 0;
    memcpy(c->id, &buf[pos], ID_SIZE);
    pos += ID_SIZE;

    c->ip_type = buf[pos++];
    size_t ip_len = (c->ip_type == 6) ? 16 : 4;
    if (len < pos + ip_len + 2) return -1;

    memset(c->ip, 0, sizeof(c->ip));
    memcpy(c->ip, &buf[pos], ip_len);
    pos += ip_len;

    c->port = read_u16_be(&buf[pos]);
    pos += 2;

    *out_read = pos;
    return 0;
}

int encode_message(const Message* msg, uint8_t* out_buf, size_t buf_len) {
    if (!msg || !out_buf) return -1;

    /* Header: Magic(2) + Type(1) + RequestId(8) + SenderId(20) = 31 bytes */
    if (buf_len < 31) return -1;

    size_t pos = 0;
    out_buf[pos++] = KADEMLIA_MAGIC_0;
    out_buf[pos++] = KADEMLIA_MAGIC_1;
    out_buf[pos++] = msg->type;
    write_u64_be(&out_buf[pos], msg->request_id);
    pos += 8;
    memcpy(&out_buf[pos], msg->sender_id, ID_SIZE);
    pos += ID_SIZE;

    switch (msg->type) {
        case MSG_PING:
        case MSG_PONG:
            break;

        case MSG_STORE: {
            if (msg->payload.store.value_len > MAX_VALUE_SIZE) return -1;
            if (buf_len < pos + ID_SIZE + 4 + msg->payload.store.value_len) return -1;
            memcpy(&out_buf[pos], msg->payload.store.key, ID_SIZE);
            pos += ID_SIZE;
            write_u32_be(&out_buf[pos], msg->payload.store.value_len);
            pos += 4;
            if (msg->payload.store.value_len > 0) {
                memcpy(&out_buf[pos], msg->payload.store.value, msg->payload.store.value_len);
                pos += msg->payload.store.value_len;
            }
            break;
        }

        case MSG_STORE_ACK: {
            if (buf_len < pos + 1) return -1;
            out_buf[pos++] = msg->payload.store_ack.status;
            break;
        }

        case MSG_FIND_NODE: {
            if (buf_len < pos + ID_SIZE) return -1;
            memcpy(&out_buf[pos], msg->payload.find_node.target_id, ID_SIZE);
            pos += ID_SIZE;
            break;
        }

        case MSG_FIND_NODE_RESP: {
            uint8_t count = msg->payload.find_node_resp.count;
            if (count > MAX_CONTACTS) count = MAX_CONTACTS;
            if (buf_len < pos + 1) return -1;
            out_buf[pos++] = count;

            for (uint8_t i = 0; i < count; ++i) {
                size_t written = 0;
                if (encode_contact(&msg->payload.find_node_resp.contacts[i], &out_buf[pos], buf_len - pos, &written) != 0) {
                    return -1;
                }
                pos += written;
            }
            break;
        }

        case MSG_FIND_VALUE: {
            if (buf_len < pos + ID_SIZE) return -1;
            memcpy(&out_buf[pos], msg->payload.find_value.key, ID_SIZE);
            pos += ID_SIZE;
            break;
        }

        case MSG_FIND_VALUE_RESP: {
            if (buf_len < pos + 1) return -1;
            out_buf[pos++] = msg->payload.find_value_resp.has_value;
            if (msg->payload.find_value_resp.has_value) {
                uint32_t vlen = msg->payload.find_value_resp.value_len;
                if (vlen > MAX_VALUE_SIZE) return -1;
                if (buf_len < pos + 4 + vlen) return -1;
                write_u32_be(&out_buf[pos], vlen);
                pos += 4;
                if (vlen > 0) {
                    memcpy(&out_buf[pos], msg->payload.find_value_resp.value, vlen);
                    pos += vlen;
                }
            } else {
                uint8_t count = msg->payload.find_value_resp.count;
                if (count > MAX_CONTACTS) count = MAX_CONTACTS;
                if (buf_len < pos + 1) return -1;
                out_buf[pos++] = count;
                for (uint8_t i = 0; i < count; ++i) {
                    size_t written = 0;
                    if (encode_contact(&msg->payload.find_value_resp.contacts[i], &out_buf[pos], buf_len - pos, &written) != 0) {
                        return -1;
                    }
                    pos += written;
                }
            }
            break;
        }

        default:
            return -3;
    }

    return (int)pos;
}

int decode_message(const uint8_t* buf, size_t len, Message* out_msg) {
    if (!buf || !out_msg) return -1;
    if (len < 31) return -2;

    if (buf[0] != KADEMLIA_MAGIC_0 || buf[1] != KADEMLIA_MAGIC_1) {
        return -2;
    }

    memset(out_msg, 0, sizeof(Message));

    size_t pos = 2;
    out_msg->type = buf[pos++];
    out_msg->request_id = read_u64_be(&buf[pos]);
    pos += 8;
    memcpy(out_msg->sender_id, &buf[pos], ID_SIZE);
    pos += ID_SIZE;

    switch (out_msg->type) {
        case MSG_PING:
        case MSG_PONG:
            break;

        case MSG_STORE: {
            if (len < pos + ID_SIZE + 4) return -4;
            memcpy(out_msg->payload.store.key, &buf[pos], ID_SIZE);
            pos += ID_SIZE;
            uint32_t vlen = read_u32_be(&buf[pos]);
            pos += 4;
            if (vlen > MAX_VALUE_SIZE || len < pos + vlen) return -4;
            out_msg->payload.store.value_len = vlen;
            if (vlen > 0) {
                memcpy(out_msg->payload.store.value, &buf[pos], vlen);
                pos += vlen;
            }
            break;
        }

        case MSG_STORE_ACK: {
            if (len < pos + 1) return -4;
            out_msg->payload.store_ack.status = buf[pos++];
            break;
        }

        case MSG_FIND_NODE: {
            if (len < pos + ID_SIZE) return -4;
            memcpy(out_msg->payload.find_node.target_id, &buf[pos], ID_SIZE);
            pos += ID_SIZE;
            break;
        }

        case MSG_FIND_NODE_RESP: {
            if (len < pos + 1) return -4;
            uint8_t count = buf[pos++];
            if (count > MAX_CONTACTS) return -4;
            out_msg->payload.find_node_resp.count = count;

            for (uint8_t i = 0; i < count; ++i) {
                size_t read_bytes = 0;
                if (decode_contact(&buf[pos], len - pos, &out_msg->payload.find_node_resp.contacts[i], &read_bytes) != 0) {
                    return -4;
                }
                pos += read_bytes;
            }
            break;
        }

        case MSG_FIND_VALUE: {
            if (len < pos + ID_SIZE) return -4;
            memcpy(out_msg->payload.find_value.key, &buf[pos], ID_SIZE);
            pos += ID_SIZE;
            break;
        }

        case MSG_FIND_VALUE_RESP: {
            if (len < pos + 1) return -4;
            uint8_t has_value = buf[pos++];
            out_msg->payload.find_value_resp.has_value = has_value;
            if (has_value) {
                if (len < pos + 4) return -4;
                uint32_t vlen = read_u32_be(&buf[pos]);
                pos += 4;
                if (vlen > MAX_VALUE_SIZE || len < pos + vlen) return -4;
                out_msg->payload.find_value_resp.value_len = vlen;
                if (vlen > 0) {
                    memcpy(out_msg->payload.find_value_resp.value, &buf[pos], vlen);
                    pos += vlen;
                }
            } else {
                if (len < pos + 1) return -4;
                uint8_t count = buf[pos++];
                if (count > MAX_CONTACTS) return -4;
                out_msg->payload.find_value_resp.count = count;
                for (uint8_t i = 0; i < count; ++i) {
                    size_t read_bytes = 0;
                    if (decode_contact(&buf[pos], len - pos, &out_msg->payload.find_value_resp.contacts[i], &read_bytes) != 0) {
                        return -4;
                    }
                    pos += read_bytes;
                }
            }
            break;
        }

        default:
            return -3;
    }

    return 0;
}
