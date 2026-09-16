#ifndef NODEX_MNEMONIC_H
#define NODEX_MNEMONIC_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define MNEMONIC_WORD_COUNT 2048
#define MNEMONIC_12_WORDS 12

/**
 * Generates a 12-word mnemonic from 16 bytes (128 bits) of entropy.
 * Returns the length of the string written to out_words_buf, or negative on error.
 */
int c_mnemonic_generate_12(const uint8_t *entropy_16, char *out_words_buf, size_t buf_len);

/**
 * Validates a 12-word mnemonic string and computes a deterministic 32-byte seed.
 * Returns 0 on success, negative if validation fails.
 */
int c_mnemonic_to_seed(const char *mnemonic_str, uint8_t *out_seed_32);

/**
 * Validates if a string contains 12 valid BIP-39 words with correct checksum.
 * Returns 1 if valid, 0 if invalid.
 */
int c_mnemonic_validate(const char *mnemonic_str);

#ifdef __cplusplus
}
#endif

#endif // NODEX_MNEMONIC_H
