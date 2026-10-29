/* See witness-format.md for host-test and RISC-V build commands. */
#include "state.h"
#include "runtime.h"
#include <assert.h>
#include <setjmp.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static jmp_buf failure;
static int trap_expected;
static unsigned char memory[4096], witness[8192], replay_hash[32];
static unsigned hash_calls;
_Noreturn void runtime_trap(void) { if(!trap_expected){fputs("unexpected runtime trap\n",stderr);abort();}longjmp(failure, 1); }
uint8_t *runtime_checked_range(uint64_t offset, uint64_t len, uint64_t end) {
    if (offset > end || len > end-offset || end > sizeof(memory)) runtime_trap();
    return memory+offset;
}
/* Independent Keccak-f reference; the known empty/abc vectors below check it. */
static uint64_t rotate(uint64_t x, unsigned n) { return n ? (x<<n)|(x>>(64-n)) : x; }
static void permutation(uint64_t s[25]) {
    static const uint64_t rc[24] = {1,0x8082,0x800000000000808aULL,0x8000000080008000ULL,0x808b,0x80000001,0x8000000080008081ULL,0x8000000000008009ULL,0x8a,0x88,0x80008009,0x8000000a,0x8000808b,0x800000000000008bULL,0x8000000000008089ULL,0x8000000000008003ULL,0x8000000000008002ULL,0x8000000000000080ULL,0x800a,0x800000008000000aULL,0x8000000080008081ULL,0x8000000000008080ULL,0x80000001,0x8000000080008008ULL};
    static const unsigned r[25] = {0,1,62,28,27,36,44,6,55,20,3,10,43,25,39,41,45,15,21,8,18,2,61,56,14};
    for (unsigned round=0; round<24; ++round) {
        uint64_t c[5],d[5],b[25];
        for (unsigned x=0;x<5;++x) c[x]=s[x]^s[x+5]^s[x+10]^s[x+15]^s[x+20];
        for (unsigned x=0;x<5;++x) d[x]=c[(x+4)%5]^rotate(c[(x+1)%5],1);
        for (unsigned y=0;y<5;++y) for(unsigned x=0;x<5;++x) b[y+5*((2*x+3*y)%5)]=rotate(s[x+5*y]^d[x],r[x+5*y]);
        for (unsigned y=0;y<5;++y) for(unsigned x=0;x<5;++x) s[x+5*y]=b[x+5*y]^((~b[(x+1)%5+5*y])&b[(x+2)%5+5*y]);
        s[0]^=rc[round];
    }
}
void runtime_keccak(const void *input, size_t len, uint8_t output[32]) {
    uint64_t s[25]={0}; const unsigned char *p=input; unsigned char *b=(void*)s;
    ++hash_calls;
    while(len>=136) { for(unsigned i=0;i<136;++i)b[i]^=*p++; permutation(s);len-=136; }
    for(unsigned i=0;i<len;++i)b[i]^=p[i];
    b[len]^=1;b[135]^=128;permutation(s);memcpy(output,s,32);
}
#ifdef __APPLE__
#include <CommonCrypto/CommonDigest.h>
#else
#include <openssl/sha.h>
#endif
void runtime_sha256(const void *input, size_t len, uint8_t output[32]) {
    assert(len<=UINT32_MAX);
#ifdef __APPLE__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    assert(CC_SHA256(input,(CC_LONG)len,output));
#pragma clang diagnostic pop
#else
    assert(SHA256(input,(size_t)len,output));
#endif
    ++hash_calls;
}
#ifdef STATE_TEST_STUB
uint64_t runtime_state(uint64_t op,const uint64_t args[8],uint64_t end) {(void)op;(void)args;(void)end;return 0;}
#endif
static void put32(unsigned char *p,uint32_t x) {for(unsigned i=0;i<4;++i)p[i]=(unsigned char)(x>>(8*i));}
static void put64(unsigned char *p,uint64_t x) {for(unsigned i=0;i<8;++i)p[i]=(unsigned char)(x>>(8*i));}
static size_t empty(void) {memset(witness,0,sizeof(witness));memcpy(witness,"NZKWIT01",8);return 180;}
static uint64_t call(uint64_t op,uint64_t a,uint64_t b,uint64_t c,uint64_t d,uint64_t e){const uint64_t args[8]={a,b,c,d,e,0,0,0};return runtime_state(op,args,sizeof(memory));}
static void init(size_t len){call(0,(uintptr_t)witness,len,(uintptr_t)replay_hash,0,0);}
#define MUST_TRAP(expr) do {trap_expected=1;if(setjmp(failure)==0){expr;fprintf(stderr,"missing trap: %s at %d\n",#expr,__LINE__);exit(1);}trap_expected=0;}while(0)
static size_t add_preimage(size_t at,unsigned type,const unsigned char key[32],const void *bytes,size_t len){witness[at++]=(unsigned char)type;memcpy(witness+at,key,32);at+=32;put32(witness+at,(uint32_t)len);at+=4;memcpy(witness+at,bytes,len);return at+len;}
static size_t add_message(size_t at,uint64_t number,const void *bytes,size_t len){put64(witness+at,number);put32(witness+at+8,(uint32_t)len);memcpy(witness+at+12,bytes,len);return at+12+len;}
static uint64_t read64(const unsigned char *p) {uint64_t n=0;for(unsigned i=0;i<8;++i)n|=(uint64_t)p[i]<<(8*i);return n;}
static void fixture_check(const char *input_path,const char *expected_path) {
    FILE *file=fopen(input_path,"rb");assert(file);assert(fseek(file,0,SEEK_END)==0);
    long file_len=ftell(file);assert(file_len>=188);rewind(file);
    unsigned char *framed=malloc((size_t)file_len);assert(framed);
    assert(fread(framed,1,(size_t)file_len,file)==(size_t)file_len);assert(fclose(file)==0);
    uint64_t len=read64(framed);assert(len<=(uint64_t)file_len-8);assert((len+15)/8*8==(uint64_t)file_len);
    const unsigned char *payload=framed+8;assert(payload[172]==1 && payload[176]==0);
    file=fopen(expected_path,"rb");assert(file);unsigned char expected[256],actual[256];
    assert(fread(expected,1,256,file)==256);assert(fgetc(file)==EOF);assert(fclose(file)==0);
    memcpy(replay_hash,expected,32);call(0,(uintptr_t)payload,len,(uintptr_t)replay_hash,0,0);
    call(14,read64(payload+152),0,0,0,0);
    /* This oracle checks encoding/claims only; real replay must derive this end state. */
    memcpy(memory+64,expected+112,64);call(11,0,64,0,0,0);call(11,1,96,0,0,0);
    call(13,0,read64(expected+176),0,0,0);call(13,1,read64(expected+184),0,0,0);
    call(1,(uintptr_t)actual,0,0,0,0);assert(memcmp(actual,expected,256)==0);free(framed);
    puts("recorded fixture witness and claim crosscheck passed");
}
static void inbox_checkpoints(void) {
    size_t len=empty();
    memset(witness+8,0x11,32);memset(witness+40,0x22,32);
    put64(witness+72,10);put64(witness+80,3);
    memset(witness+88,0x55,32);memset(witness+120,0x66,32);
    put64(witness+152,10);put64(witness+160,7);
    put32(witness+172,4);put32(witness+176,2);
    const unsigned char checkpoints[4]={7,8,8,9};
    for(unsigned i=0;i<4;++i) {
        unsigned char seq[45];memset(seq,10+i,32);memset(seq+32,0,8);
        seq[39]=checkpoints[i];memcpy(seq+40,"batch",5);len=add_message(len,10+i,seq,sizeof(seq));
    }
    for(unsigned i=0;i<2;++i) {
        unsigned char delayed[120];memset(delayed,7+i,113);memcpy(delayed+113,"delayed",7);
        len=add_message(len,7+i,delayed,sizeof(delayed));
    }
    init(len);hash_calls=0;
    assert(call(14,13,32,64,0,0)==13);assert(call(15,8,96,64,0,0)==24);
    call(14,10,0,64,0,0);call(15,7,0,64,0,0);call(14,12,0,64,0,0);call(14,11,0,64,0,0);
    call(15,8,0,64,0,0);call(14,13,0,64,0,0);call(15,7,32,64,0,0);
    call(13,0,14,0,0,0);call(13,1,0,0,0,0);
    unsigned char actual[256],expected[256];call(1,(uintptr_t)actual,0,0,0,0);
    /* Four sequencer hashes, two delayed hashes each, one fold each and two claims. */
    assert(hash_calls==16);
    memcpy(expected,replay_hash,32);memcpy(expected+32,witness+8,80);memcpy(expected+112,witness+8,80);
    put64(expected+176,14);put64(expected+184,0);
    /* Fixed vectors from the independent Python Keccak oracle; also checked in the real VM. */
    const char *claims="4b2844598ca14e3045e4f9831051ea86eae89d37cea5740b3ae74bb1f329319a"
                       "ae88c07bb601f53c3b9d256d777f461804fc623ffdb2d121d32f14450aef84e3";
    for(unsigned i=0;i<64;++i) {unsigned byte;assert(sscanf(claims+2*i,"%2x",&byte)==1);expected[192+i]=(unsigned char)byte;}
    assert(memcmp(actual,expected,256)==0);
    unsigned char seq[40]={0},delayed[114]={0};
    /* Zero cannot name a snapshot before the opening count. */
    len=empty();put64(witness+160,7);put32(witness+172,1);len=add_message(len,0,seq,40);
    init(len);call(14,0,0,64,0,0);MUST_TRAP(call(1,(uintptr_t)actual,0,0,0,0));
    /* A later batch cannot move the checkpoint backwards, including to zero. */
    len=empty();put32(witness+172,2);put32(witness+176,1);seq[39]=1;
    len=add_message(len,0,seq,40);seq[39]=0;len=add_message(len,1,seq,40);
    len=add_message(len,0,delayed,114);init(len);
    call(14,0,0,64,0,0);call(14,1,0,64,0,0);call(15,0,0,64,0,0);
    MUST_TRAP(call(1,(uintptr_t)actual,0,0,0,0));
    /* Reaching an intermediate checkpoint does not cover the consumed delayed end. */
    len=empty();put32(witness+172,1);put32(witness+176,2);seq[39]=1;
    len=add_message(len,0,seq,40);len=add_message(len,0,delayed,114);len=add_message(len,1,delayed,114);
    init(len);call(14,0,0,64,0,0);call(15,0,0,64,0,0);call(15,1,0,64,0,0);
    MUST_TRAP(call(1,(uintptr_t)actual,0,0,0,0));
    len=empty();put64(witness+160,UINT64_MAX);put32(witness+172,1);put32(witness+176,1);
    memset(seq+32,0xff,8);len=add_message(len,0,seq,40);len=add_message(len,UINT64_MAX,delayed,114);
    init(len);call(14,0,0,64,0,0);call(15,UINT64_MAX,0,64,0,0);
    MUST_TRAP(call(1,(uintptr_t)actual,0,0,0,0));
}
int main(int argc,char **argv) {
    unsigned char hash[32]; runtime_keccak("",0,hash);
    static const unsigned char empty_hash[32]={0xc5,0xd2,0x46,0x01,0x86,0xf7,0x23,0x3c,0x92,0x7e,0x7d,0xb2,0xdc,0xc7,0x03,0xc0,0xe5,0x00,0xb6,0x53,0xca,0x82,0x27,0x3b,0x7b,0xfa,0xd8,0x04,0x5d,0x85,0xa4,0x70};
    assert(memcmp(hash,empty_hash,32)==0);
    static const unsigned char sha_abc[32]={0xba,0x78,0x16,0xbf,0x8f,0x01,0xcf,0xea,0x41,0x41,0x40,0xde,0x5d,0xae,0x22,0x23,0xb0,0x03,0x61,0xa3,0x96,0x17,0x7a,0x9c,0xb4,0x10,0xff,0x61,0xf2,0x00,0x15,0xad};
    runtime_sha256("abc",3,hash);assert(memcmp(hash,sha_abc,32)==0);
    size_t len=empty();init(len);
    MUST_TRAP(init(len-1)); MUST_TRAP(init(len+1));witness[0]^=1;MUST_TRAP(init(len));
    len=empty();put32(witness+168,STATE_MAX_PREIMAGES+1);MUST_TRAP(init(len));
    len=empty();init(len);MUST_TRAP(call(10,2,0,0,0,0));MUST_TRAP(call(10,0,4090,0,0,0));
    call(13,1,UINT64_MAX,0,0,0);assert(call(12,1,0,0,0,0)==UINT64_MAX);
    memset(memory+64,0x76,32);call(11,1,64,0,0,0);call(10,1,96,0,0,0);assert(memcmp(memory+64,memory+96,32)==0);
    MUST_TRAP(call(14,0,0,0,0,0));MUST_TRAP(call(16,0,0,0,32,0));
    /* Omission must not turn unsupported CustomDA authentication into an empty batch. */
    MUST_TRAP(call(17,3,0,0,0,0));
    for(unsigned type=0;type<4;++type)MUST_TRAP(call(17,type,0,0,0,0));
    MUST_TRAP(call(17,0,0,UINT64_MAX,UINT64_MAX,UINT64_MAX));
    runtime_keccak("abc",3,hash);memcpy(memory,hash,32);len=empty();put32(witness+168,1);len=add_preimage(len,0,hash,"abc",3);init(len);hash_calls=0;
    assert(call(16,0,0,0,32,0)==3);assert(memcmp(memory+32,"abc",3)==0);assert(hash_calls==1);
    MUST_TRAP(call(17,0,0,0,0,0));assert(hash_calls==1);MUST_TRAP(call(17,256,0,0,0,0));MUST_TRAP(call(17,0,0,UINT64_MAX,UINT64_MAX,UINT64_MAX));
    MUST_TRAP(call(16,0,0,1,32,0));assert(call(16,0,0,32,32,0)==0);
    assert(call(18,0,0,1,1,64)==2);assert(memory[64]=='b');
    witness[len-1]^=1;init(len);MUST_TRAP(call(16,0,0,0,32,0));
    len=empty();put32(witness+168,2);len=add_preimage(len,0,hash,"abc",3);len=add_preimage(len,0,hash,"abc",3);MUST_TRAP(init(len));
    for(unsigned type=2;type<=3;++type){len=empty();put32(witness+168,1);len=add_preimage(len,type,hash,"abc",3);init(len);MUST_TRAP(call(17,type,0,0,0,0));}
    len=empty();put32(witness+168,1);runtime_sha256("abc",3,hash);memcpy(memory,hash,32);len=add_preimage(len,1,hash,"abc",3);init(len);assert(call(16,1,0,0,32,0)==3);MUST_TRAP(call(17,257,0,0,0,0));
    unsigned char seq[40]={0},delayed[114]={0},output[256];
    len=empty();put32(witness+172,1);len=add_message(len,1,seq,sizeof(seq));init(len);assert(call(14,1,0,0,0,0)==32);MUST_TRAP(call(1,(uintptr_t)output,0,0,0,0));
    len=empty();put32(witness+172,2);len=add_message(len,0,seq,sizeof(seq));len=add_message(len,0,seq,sizeof(seq));MUST_TRAP(init(len));
    len=empty();put32(witness+172,1);len=add_message(len,UINT64_MAX,seq,sizeof(seq));put64(witness+152,UINT64_MAX);init(len);call(14,UINT64_MAX,0,0,0,0);MUST_TRAP(call(1,(uintptr_t)output,0,0,0,0));
    len=empty();put32(witness+176,1);len=add_message(len,0,delayed,sizeof(delayed));init(len);call(15,0,0,0,0,0);MUST_TRAP(call(1,(uintptr_t)output,0,0,0,0));
    len=empty();put32(witness+172,1);put32(witness+176,1);seq[39]=2;len=add_message(len,0,seq,sizeof(seq));len=add_message(len,0,delayed,sizeof(delayed));init(len);call(14,0,0,0,0,0);call(15,0,0,0,0,0);MUST_TRAP(call(1,(uintptr_t)output,0,0,0,0));
    seq[39]=1;len=empty();put32(witness+172,1);put32(witness+176,1);len=add_message(len,0,seq,sizeof(seq));len=add_message(len,0,delayed,sizeof(delayed));init(len);call(14,0,0,0,0,0);call(14,0,32,0,0,0);call(15,0,0,0,0,0);call(1,(uintptr_t)output,0,0,0,0);assert(memcmp(output,replay_hash,32)==0);
    /* Independent byte layout for both claim domains, including exactly one fold. */
    unsigned char delayed_data_hash[32],delayed_message_hash[32],delayed_acc[32],seq_hash[32],seq_acc[32],packed[248]={0},expected[32];
    runtime_keccak(delayed+113,sizeof(delayed)-113,delayed_data_hash);
    unsigned char delayed_packed[133]={0};memcpy(delayed_packed+101,delayed_data_hash,32);
    runtime_keccak(delayed_packed,133,delayed_message_hash);
    memcpy(packed+32,delayed_message_hash,32);runtime_keccak(packed,64,delayed_acc);
    memset(packed,0,sizeof(packed));runtime_keccak(seq,40,seq_hash);memcpy(packed+32,seq_hash,32);memcpy(packed+64,delayed_acc,32);runtime_keccak(packed,96,seq_acc);
    memset(packed,0,sizeof(packed));memcpy(packed,"ARBZSEQ1",8);memcpy(packed+200,seq_acc,32);put64(packed+240,1);runtime_keccak(packed,248,expected);assert(memcmp(output+192,expected,32)==0);
    memcpy(packed,"ARBZDEL1",8);memcpy(packed+200,delayed_acc,32);runtime_keccak(packed,248,expected);assert(memcmp(output+224,expected,32)==0);
    MUST_TRAP(call(1,(uintptr_t)output,0,0,0,0));
    len=empty();init(len);MUST_TRAP(call(1,UINT64_MAX,0,0,0,0));
    len=empty();put32(witness+168,1);memset(hash,0,32);len=add_preimage(len,1,hash,"abc",3);memcpy(memory,hash,32);init(len);MUST_TRAP(call(16,1,0,0,32,0));
    len=empty();put32(witness+168,1);len=add_preimage(len,0,hash,"abc",3);put32(witness+213,UINT32_MAX);MUST_TRAP(init(len));
    len=empty();put32(witness+172,1);len=add_message(len,0,seq,39);init(len);MUST_TRAP(call(14,0,0,0,0,0));
    len=empty();put32(witness+176,1);len=add_message(len,0,delayed,112);init(len);MUST_TRAP(call(15,0,0,0,0,0));
    len=empty();put32(witness+172,2);seq[39]=1;len=add_message(len,0,seq,40);seq[39]=2;len=add_message(len,1,seq,40);put32(witness+176,1);len=add_message(len,0,delayed,114);init(len);call(14,0,0,0,0,0);call(14,1,0,0,0,0);call(15,0,0,0,0,0);MUST_TRAP(call(1,(uintptr_t)output,0,0,0,0));
    len=empty();put32(witness+172,1);seq[39]=1;len=add_message(len,0,seq,40);init(len);call(14,0,0,0,0,0);MUST_TRAP(call(1,(uintptr_t)output,0,0,0,0));
    /* Unused optional witness entries do not contribute to the consumed window. */
    len=empty();put32(witness+172,1);len=add_message(len,9,seq,40);init(len);call(1,(uintptr_t)output,0,0,0,0);
    len=empty();init(len);MUST_TRAP(call(10,0,UINT64_C(0x100000000),0,0,0));
    MUST_TRAP(init(UINT64_MAX));MUST_TRAP(call(12,0,0,0,0,0));
    inbox_checkpoints();
    puts("state runtime tests passed");if(argc==3)fixture_check(argv[1],argv[2]);else assert(argc==1);return 0;
}
