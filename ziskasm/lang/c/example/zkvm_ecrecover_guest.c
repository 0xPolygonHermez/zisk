#include "zkvm_accelerators.h"
#include "emit.h"
static const zkvm_secp256k1_hash MSG = {{17,17,17,17,34,34,34,34,51,51,51,51,68,68,68,68,85,85,85,85,102,102,102,102,119,119,119,119,136,136,136,136}};
static const zkvm_secp256k1_signature SIG = {{84,36,223,85,198,73,54,225,234,38,254,27,175,201,247,190,203,9,180,224,139,173,176,95,61,90,57,155,17,175,86,206,98,28,89,239,113,15,176,34,16,102,217,175,0,191,72,45,211,212,153,145,242,175,168,247,69,168,138,92,159,88,117,82}};
static const uint8_t EXP[64] = {117,242,22,82,115,211,108,70,3,218,159,76,58,243,15,164,247,152,211,216,128,213,51,138,204,207,28,251,131,153,1,124,60,0,108,47,92,136,74,76,66,241,6,9,85,27,85,178,20,219,207,162,244,7,166,209,60,181,63,127,140,9,252,133};
static uint8_t g_out[32];
static volatile uint32_t g_dv = 0x11223344;
/* 1 if ecrecover accepts `recid` for (MSG, SIG), 0 if it returns ZKVM_EFAIL. The
   recovered key is irrelevant here -- only the accepted id range is under test. */
static uint8_t ecrecover_ok(uint8_t recid){
    zkvm_secp256k1_pubkey p;
    return zkvm_secp256k1_ecrecover(&MSG, &SIG, recid, &p) == ZKVM_EOK ? 1 : 0;
}

int main(void){
    zkvm_secp256k1_pubkey pk;
    zkvm_status st = zkvm_secp256k1_ecrecover(&MSG, &SIG, 1, &pk);
    int match = 1; for (int i=0;i<64;i++) if (pk.data[i]!=EXP[i]) match=0;
    g_out[0] = (st==ZKVM_EOK)?1:0;
    g_out[1] = match?1:0;
    g_out[2] = (uint8_t)g_dv;
    /* Recovery-id range. ZisK accepts only 0 and 1: `ecdsa_recover_secp256k1`
       rejects recid > 1 (ECDSA_ERR_INVALID_RECID) and zkvm/secp256k1.zisk mirrors
       that guard, so the two backends agree. Ids 2/3 are the x = r + n branch,
       which needs the R point's x in [n, p) -- about a 2^-128 window for secp256k1
       -- and Ethereum's ecrecover only ever passes v=27/28, i.e. recid 0/1.
       Pinned here so the restriction is a tested contract, not an assumption.
       Expect bytes 3..5 = 01 00 00. */
    g_out[3] = ecrecover_ok(0);   /* valid: even-y R          -> 1 */
    g_out[4] = ecrecover_ok(2);   /* x = r + n branch          -> 0 */
    g_out[5] = ecrecover_ok(3);   /* x = r + n, odd y          -> 0 */
    emit32(g_out);
    return 0;
}
