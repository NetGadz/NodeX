#ifndef KADEMLIA_HASH_H
#define KADEMLIA_HASH_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Returns the number of consecutive leading bits that match between id1 and id2.
 * Used to calculate the shared prefix length and k-bucket index in Kademlia.
 */
uint32_t c_shared_prefix_bits(const uint8_t* id1, const uint8_t* id2, size_t len);

/*
 * Computes SHA-1 hash of input bytes, outputting a 20-byte (160-bit) identifier.
 */
void hash_node_id(const uint8_t* input, size_t len, uint8_t out_id[20]);

#ifdef __cplusplus
}
#endif

#endif /* KADEMLIA_HASH_H */
