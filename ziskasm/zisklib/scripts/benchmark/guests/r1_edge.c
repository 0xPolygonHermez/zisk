#include "zkvm_accelerators.h"
static const zkvm_secp256r1_hash MSG = {{222,173,190,239,17,34,51,68,85,102,119,136,153,0,17,34,51,68,85,102,119,136,153,0,170,187,204,221,238,255,0,17}};
static const zkvm_secp256r1_signature SIG = {{152,66,37,88,93,34,133,193,56,3,61,97,64,227,206,248,185,24,89,112,78,83,195,19,248,182,54,186,79,150,118,73,22,43,235,76,80,214,16,37,15,176,193,119,121,166,130,201,117,187,63,100,199,173,239,181,184,114,83,140,160,157,241,39}};
static const zkvm_secp256r1_pubkey PUB = {{81,92,61,110,185,227,150,185,4,211,254,202,127,84,253,205,12,193,233,151,191,55,93,202,81,90,208,166,195,180,3,95,69,54,190,58,80,243,24,251,249,165,71,89,2,162,33,80,43,239,13,87,224,140,83,178,204,10,86,241,125,159,147,84}};
static const uint8_t N[32] = {0xff,0xff,0xff,0xff,0x00,0x00,0x00,0x00,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xbc,0xe6,0xfa,0xad,0xa7,0x17,0x9e,0x84,0xf3,0xb9,0xca,0xc2,0xfc,0x63,0x25,0x51};
static const uint8_t P[32] = {0xff,0xff,0xff,0xff,0x00,0x00,0x00,0x01,0,0,0,0,0,0,0,0,0,0,0,0,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff};
static void set(uint8_t *d, const uint8_t *s) { for (int i = 0; i < 32; i++) d[i] = s[i]; }
static void setv(uint8_t *d, int v) { for (int i = 0; i < 32; i++) d[i] = (uint8_t)v; }
static uint8_t o[256]; static unsigned k;
static void ver(const zkvm_secp256r1_signature *sg, const zkvm_secp256r1_pubkey *pk) {
    bool ok = 0; o[k++] = (uint8_t)zkvm_secp256r1_verify(&MSG, sg, pk, &ok); o[k++] = ok; }
int main(void) {
    zkvm_secp256r1_signature sg; zkvm_secp256r1_pubkey pk;
    ver(&SIG, &PUB);
    for (int w = 0; w < 2; w++)
        for (int c = 0; c < 9; c++) {
            sg = SIG; uint8_t *x = sg.data + 32 * w;
            if (c == 0) setv(x, 0);
            if (c == 1) { setv(x, 0); x[31] = 1; }
            if (c == 2) { set(x, N); x[31] -= 1; }
            if (c == 3) set(x, N);
            if (c == 4) { set(x, N); x[31] += 1; }
            if (c == 5) setv(x, 0xff);
            if (c == 6) { setv(x, 0); x[31] = 5; x[20] = 9; }
            if (c == 7) { set(x, N); x[7] = 0xff; }
            if (c == 8) { set(x, N); x[3] = 0xfe; }
            ver(&sg, &PUB);
        }
    for (int c = 0; c < 8; c++) {
        pk = PUB;
        if (c == 0) setv(pk.data, 0);
        if (c == 1) setv(pk.data, 0), pk.data[63] = 1;
        if (c == 2) set(pk.data, P);
        if (c == 3) set(pk.data + 32, P);
        if (c == 4) setv(pk.data, 0xff);
        if (c == 5) pk.data[63] ^= 1;
        if (c == 6) { setv(pk.data, 0); pk.data[40] = 3; }
        if (c == 7) { set(pk.data, P); pk.data[31] -= 1; }
        ver(&SIG, &pk);
    }
    volatile uint8_t *O = (volatile uint8_t *)0xA0410000ULL;
    for (unsigned i = 0; i < k; i++) O[i] = o[i];
    return 0;
}
