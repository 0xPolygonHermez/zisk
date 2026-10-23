#include "zkvm_accelerators.h"
static const zkvm_secp256k1_hash MSG = {{171,205,239,0,17,34,51,68,85,102,119,136,153,170,187,204,221,238,255,0,17,34,51,68,85,102,119,136,153,170,187,204}};
static const zkvm_secp256k1_signature SIG = {{63,16,95,167,152,80,122,104,66,76,223,108,169,71,134,230,178,195,73,201,106,170,101,172,72,181,236,113,4,63,30,49,11,165,194,244,46,173,236,14,56,234,89,75,65,16,167,93,74,82,196,139,95,216,148,226,0,114,28,167,32,144,62,246}};
static const zkvm_secp256k1_pubkey PUB = {{156,72,64,6,140,141,175,171,104,173,211,253,154,105,200,7,218,144,21,83,56,227,206,147,110,169,152,240,76,122,233,128,111,127,57,210,169,10,198,201,133,124,19,167,220,115,133,156,34,121,222,13,114,36,63,118,91,223,217,201,101,31,9,208}};
static const zkvm_secp256k1_hash RMSG = {{17,17,17,17,34,34,34,34,51,51,51,51,68,68,68,68,85,85,85,85,102,102,102,102,119,119,119,119,136,136,136,136}};
static const zkvm_secp256k1_signature RSIG = {{84,36,223,85,198,73,54,225,234,38,254,27,175,201,247,190,203,9,180,224,139,173,176,95,61,90,57,155,17,175,86,206,98,28,89,239,113,15,176,34,16,102,217,175,0,191,72,45,211,212,153,145,242,175,168,247,69,168,138,92,159,88,117,82}};
/* Edge cases for the secp256k1 verify/ecrecover range checks and marshalling.
   Output: one status byte and one result byte per case; compared to a snapshot. */
static const uint8_t N[32] = {0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xfe,0xba,0xae,0xdc,0xe6,0xaf,0x48,0xa0,0x3b,0xbf,0xd2,0x5e,0x8c,0xd0,0x36,0x41,0x41};
static const uint8_t P[32] = {0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xfe,0xff,0xff,0xfc,0x2f};
static void set(uint8_t *d, const uint8_t *s) { for (int i = 0; i < 32; i++) d[i] = s[i]; }
static void setv(uint8_t *d, int v) { for (int i = 0; i < 32; i++) d[i] = (uint8_t)v; }
static void addk(uint8_t *d, int k) { int c = k; for (int i = 31; i >= 0 && c; i--) { int t = d[i] + c; d[i] = (uint8_t)t; c = t >> 8; if (k < 0 && t < 0) { d[i] = (uint8_t)(t + 256); c = -1; } else if (k < 0) c = 0; } }
static uint8_t o[256]; static unsigned k;
static void ver(const zkvm_secp256k1_signature *sg, const zkvm_secp256k1_pubkey *pk) {
    bool ok = 0; o[k++] = (uint8_t)zkvm_secp256k1_verify(&MSG, sg, pk, &ok); o[k++] = ok; }
static void rec(const zkvm_secp256k1_signature *sg, uint8_t id) {
    zkvm_secp256k1_pubkey p; for (int i = 0; i < 64; i++) p.data[i] = 0x5a;
    o[k++] = (uint8_t)zkvm_secp256k1_ecrecover(&RMSG, sg, id, &p);
    uint8_t h = 0; for (int i = 0; i < 64; i++) h = (uint8_t)(h * 31 + p.data[i]); o[k++] = h; }
int main(void) {
    zkvm_secp256k1_signature sg; zkvm_secp256k1_pubkey pk;
    ver(&SIG, &PUB);
    /* r / s edge values: 0, 1, n-1, n, n+1, 2^256-1, top limb 0 with low bits */
    for (int w = 0; w < 2; w++) {
        const zkvm_secp256k1_signature *base[2] = {&SIG, &RSIG};
        for (int c = 0; c < 8; c++) {
            sg = *base[0];
            uint8_t *x = sg.data + 32 * w;
            if (c == 0) setv(x, 0);
            if (c == 1) { setv(x, 0); x[31] = 1; }
            if (c == 2) { set(x, N); addk(x, -1); }
            if (c == 3) set(x, N);
            if (c == 4) { set(x, N); addk(x, 1); }
            if (c == 5) setv(x, 0xff);
            if (c == 6) { setv(x, 0); x[31] = 5; x[20] = 9; }
            if (c == 7) { set(x, N); x[15] = 0xfd; }
            ver(&sg, &PUB);
            sg = *base[1]; x = sg.data + 32 * w;
            if (c == 0) setv(x, 0);
            if (c == 1) { setv(x, 0); x[31] = 1; }
            if (c == 2) { set(x, N); addk(x, -1); }
            if (c == 3) set(x, N);
            if (c == 4) { set(x, N); addk(x, 1); }
            if (c == 5) setv(x, 0xff);
            if (c == 6) { setv(x, 0); x[31] = 5; x[20] = 9; }
            if (c == 7) { set(x, N); x[15] = 0xfd; }
            rec(&sg, 0); rec(&sg, 1);
        }
    }
    for (int id = 0; id < 4; id++) rec(&RSIG, (uint8_t)id);
    /* pubkey edge values */
    for (int c = 0; c < 7; c++) {
        pk = PUB;
        if (c == 0) setv(pk.data, 0);
        if (c == 1) setv(pk.data, 0), pk.data[63] = 1;
        if (c == 2) set(pk.data, P);
        if (c == 3) set(pk.data + 32, P);
        if (c == 4) setv(pk.data, 0xff);
        if (c == 5) pk.data[63] ^= 1;
        if (c == 6) { setv(pk.data, 0); pk.data[40] = 3; }
        ver(&SIG, &pk);
    }
    volatile uint8_t *O = (volatile uint8_t *)0xA0410000ULL;
    for (unsigned i = 0; i < k; i++) O[i] = o[i];
    return 0;
}
