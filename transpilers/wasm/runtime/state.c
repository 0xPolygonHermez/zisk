#include "state.h"
#include "runtime.h"
#include <stddef.h>
#include <string.h>

typedef struct {
    const uint8_t *key, *bytes;
    uint32_t len;
    uint8_t type, verified;
} Preimage;
typedef struct {
    const uint8_t *bytes;
    uint64_t number, after_delayed;
    uint32_t len;
    uint8_t used, hash[32];
} Message;
static Preimage preimages[STATE_MAX_PREIMAGES];
static Message sequencer[STATE_MAX_MESSAGES], delayed[STATE_MAX_MESSAGES];
static uint32_t preimage_count, seq_count, delayed_count;
static uint8_t start_state[80], end_state[80], replay_hash[32];
static uint8_t seq_before[32], delayed_before[32];
static uint64_t seq_start, delayed_start;
static uint8_t initialized, finalized;

typedef struct { const uint8_t *next; uint64_t left; } Reader;
static const uint8_t *take(Reader *r, uint64_t len) {
    if (len > r->left) runtime_trap();
    const uint8_t *p=r->next; r->next+=len; r->left-=len; return p;
}
static uint32_t le32(const uint8_t *p) {
    return (uint32_t)p[0]|((uint32_t)p[1]<<8)|((uint32_t)p[2]<<16)|((uint32_t)p[3]<<24);
}
static uint64_t le64(const uint8_t *p) {
    uint64_t n=0;for(unsigned i=0;i<8;++i)n|=(uint64_t)p[i]<<(8*i);return n;
}
static uint64_t be64(const uint8_t *p) {
    uint64_t n=0;for(unsigned i=0;i<8;++i)n=(n<<8)|p[i];return n;
}
static void put64(uint8_t *p,uint64_t n) {
    for(unsigned i=0;i<8;++i)p[i]=(uint8_t)(n>>(8*i));
}
static uint32_t read32(Reader *r) { return le32(take(r,4)); }
static void parse_messages(Reader *r,Message *messages,uint32_t count) {
    for(uint32_t i=0;i<count;++i) {
        Message *m=&messages[i];m->number=le64(take(r,8));m->len=read32(r);
        m->bytes=take(r,m->len);m->used=0;m->after_delayed=0;
        if(i && messages[i-1].number>=m->number)runtime_trap();
    }
}
static void initialize(uint64_t pointer,uint64_t len,uint64_t hash_pointer) {
    initialized=0;finalized=0;
    if(!pointer || len>UINTPTR_MAX-pointer || !hash_pointer || hash_pointer>UINTPTR_MAX-32)runtime_trap();
    #ifndef NITRO_HOST_TEST
    if(pointer!=NITRO_INPUT_DATA || hash_pointer!=NITRO_MODULE_HASH || len>64ULL*1024*1024)runtime_trap();
#endif
    Reader r={(const uint8_t *)(uintptr_t)pointer,len};
    if(memcmp(take(&r,8),"NZKWIT01",8))runtime_trap();
    memcpy(start_state,take(&r,80),80);memcpy(end_state,start_state,80);
    memcpy(seq_before,take(&r,32),32);memcpy(delayed_before,take(&r,32),32);
    seq_start=le64(take(&r,8));delayed_start=le64(take(&r,8));
    preimage_count=read32(&r);seq_count=read32(&r);delayed_count=read32(&r);
    if(preimage_count>STATE_MAX_PREIMAGES || seq_count>STATE_MAX_MESSAGES || delayed_count>STATE_MAX_MESSAGES)runtime_trap();
    for(uint32_t i=0;i<preimage_count;++i) {
        Preimage *p=&preimages[i];p->type=*take(&r,1);p->key=take(&r,32);
        p->len=read32(&r);p->bytes=take(&r,p->len);p->verified=0;
        if(i) {
            const Preimage *prev=&preimages[i-1];
            if(prev->type>p->type || (prev->type==p->type && memcmp(prev->key,p->key,32)>=0))runtime_trap();
        }
    }
    parse_messages(&r,sequencer,seq_count);parse_messages(&r,delayed,delayed_count);
    if(r.left)runtime_trap();
    memcpy(replay_hash,(const void *)(uintptr_t)hash_pointer,32);initialized=1;
}
static Preimage *find_preimage(uint8_t type,const uint8_t key[32]) {
    uint32_t lo=0,hi=preimage_count;
    while(lo<hi) {
        uint32_t mid=lo+(hi-lo)/2;Preimage *p=&preimages[mid];
        int cmp=p->type<type?-1:p->type>type?1:memcmp(p->key,key,32);
        if(cmp<0)lo=mid+1;else if(cmp>0)hi=mid;else {
            if(!p->verified) {
                uint8_t hash[32];
                if(type==0)runtime_keccak(p->bytes,p->len,hash);
                else if(type==1)runtime_sha256(p->bytes,p->len,hash);
                else runtime_trap();
                if(memcmp(hash,p->key,32))runtime_trap();
                p->verified=1;
            }
            return p;
        }
    }
    return NULL;
}
static Message *find_message(Message *messages,uint32_t count,uint64_t number,int is_delayed) {
    uint32_t lo=0,hi=count;
    while(lo<hi) {
        uint32_t mid=lo+(hi-lo)/2;Message *m=&messages[mid];
        if(m->number<number)lo=mid+1;else if(m->number>number)hi=mid;else {
            if(!m->used) {
                if(is_delayed) {
                    if(m->len<113)runtime_trap();
                    uint8_t packed[133];packed[0]=m->bytes[0];
                    memcpy(packed+1,m->bytes+13,20);memcpy(packed+21,m->bytes+33,80);
                    runtime_keccak(m->bytes+113,m->len-113,packed+101);
                    runtime_keccak(packed,sizeof(packed),m->hash);
                } else {
                    if(m->len<40)runtime_trap();
                    m->after_delayed=be64(m->bytes+32);runtime_keccak(m->bytes,m->len,m->hash);
                }
                m->used=1;
            }
            return m;
        }
    }
    runtime_trap();
}
static uint32_t copy_chunk(const uint8_t *bytes,uint32_t len,uint32_t offset,uint32_t available,uint64_t out,uint64_t mem_end) {
    uint32_t remaining=offset<len?len-offset:0;
    uint32_t copied=remaining<available?remaining:available;
    uint8_t *destination=runtime_checked_range(out,copied,mem_end);
    if(copied)memcpy(destination,bytes+offset,copied);
    return remaining;
}
static uint64_t increment(uint64_t value) { if(value==UINT64_MAX)runtime_trap();return value+1; }
static void claim(uint8_t out[32],const char domain[8],const uint8_t before[32],const uint8_t after[32],uint64_t from,uint64_t to) {
    uint8_t packed[248];memcpy(packed,domain,8);memcpy(packed+8,start_state,80);
    memcpy(packed+88,end_state,80);memcpy(packed+168,before,32);memcpy(packed+200,after,32);
    put64(packed+232,from);put64(packed+240,to);runtime_keccak(packed,sizeof(packed),out);
}
static void finalize(uint64_t out_pointer) {
    if(!out_pointer || out_pointer>UINTPTR_MAX-256)runtime_trap();
    #ifndef NITRO_HOST_TEST
    if(out_pointer!=NITRO_OUTPUT)runtime_trap();
#endif
    uint8_t delayed_acc[32],seq_acc[32],packed[96];
    memcpy(delayed_acc,delayed_before,32);memcpy(seq_acc,seq_before,32);
    uint64_t delayed_end=delayed_start,seq_end=seq_start;int consumed_delayed=0;
    for(uint32_t i=0;i<delayed_count;++i)if(delayed[i].used) {
        if(delayed[i].number!=delayed_end)runtime_trap();delayed_end=increment(delayed_end);consumed_delayed=1;
        memcpy(packed,delayed_acc,32);memcpy(packed+32,delayed[i].hash,32);runtime_keccak(packed,64,delayed_acc);
    }
    uint64_t snapshot_count=consumed_delayed?delayed_end:delayed_start;
    const uint8_t *snapshot=consumed_delayed?delayed_acc:delayed_before;
    int has_delayed_snapshot=0;
    /* shortcut: one delayed checkpoint per replay, extend with authenticated snapshots for multi-checkpoint ranges. */
    for(uint32_t i=0;i<seq_count;++i)if(sequencer[i].used) {
        Message *m=&sequencer[i];if(m->number!=seq_end)runtime_trap();seq_end=increment(seq_end);
        memcpy(packed,seq_acc,32);memcpy(packed+32,m->hash,32);
        if(m->after_delayed) {
            if(m->after_delayed!=snapshot_count)runtime_trap();has_delayed_snapshot=1;memcpy(packed+64,snapshot,32);
        } else memset(packed+64,0,32);
        runtime_keccak(packed,96,seq_acc);
    }
    if(consumed_delayed && !has_delayed_snapshot)runtime_trap();
    uint8_t *out=(uint8_t *)(uintptr_t)out_pointer;
    memcpy(out,replay_hash,32);memcpy(out+32,start_state,80);memcpy(out+112,end_state,80);
    claim(out+192,"ARBZSEQ1",seq_before,seq_acc,seq_start,seq_end);
    claim(out+224,"ARBZDEL1",delayed_before,delayed_acc,delayed_start,delayed_end);finalized=1;
}
uint64_t runtime_state(uint64_t op,const uint64_t args[8],uint64_t mem_end) {
    uint64_t a0=args[0],a1=args[1],a2=args[2],a3=args[3],a4=args[4];
    if(op==0) {initialize(a0,a1,a2);return 0;}
    if(!initialized || finalized)runtime_trap();
    if(op==1) {finalize(a0);return 0;}
    if(op>=10 && op<=13) {
        if(a0>1)runtime_trap();
        if(op==10)memcpy(runtime_checked_range(a1,32,mem_end),end_state+a0*32,32);
        if(op==11)memcpy(end_state+a0*32,runtime_checked_range(a1,32,mem_end),32);
        if(op==12)return le64(end_state+64+a0*8);
        if(op==13)put64(end_state+64+a0*8,a1);
        return 0;
    }
    if(op==14 || op==15) {
        if(a1>UINT32_MAX)runtime_trap();
        Message *m=op==14?find_message(sequencer,seq_count,a0,0):find_message(delayed,delayed_count,a0,1);
        uint32_t remaining=copy_chunk(m->bytes,m->len,(uint32_t)a1,32,a2,mem_end);
        return remaining<32?remaining:32;
    }
    if(op==16 || op==17 || op==18) {
        if((op!=17 && a2>UINT32_MAX) || (op==18 && a3>UINT32_MAX))runtime_trap();
        const uint8_t *key=runtime_checked_range(a1,32,mem_end);Preimage *p=find_preimage((uint8_t)a0,key);
        if(op==17)return p?1:0;
        if(!p || (op==16 && a2%32))runtime_trap();
        uint32_t remaining=copy_chunk(p->bytes,p->len,(uint32_t)a2,op==18?(uint32_t)a3:32,op==18?a4:a3,mem_end);
        return op==18?remaining:remaining<32?remaining:32;
    }
    runtime_trap();
}
