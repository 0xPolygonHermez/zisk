// C++ static-initialization regression test for the program-segment based ELF
// interpreter (`transpilers/common/src/elf_extraction.rs`).
//
// A C++ translation unit with static initializers produces ELF structures that
// a plain assembly test never exercises:
//
//   * `.init_array` / `.fini_array` (`INIT_ARRAY` / `FINI_ARRAY` sections) that
//     the linker script places in the read-only ROM `PT_LOAD` segment, so the
//     constructor function pointers must be loadable from ROM at run time;
//   * `.rodata` reads from a constructor (vtables, string literals, const
//     tables) — again ROM loads;
//   * relocated pointers in `.data` (vtable pointers, pointers between globals);
//   * `.bss` guard variables for function-local statics, which only work if the
//     `p_memsz > p_filesz` zero-fill tail of the writable segment is honoured.
//
// Every check below writes one word to the ZisK output area, so the emulator's
// output is a full trace of what ran and in which order. The expected sequence
// is asserted by `emulator/tests/cpp_static_init.rs`; keep the two in sync.
//
// Deliberately self-contained (own `_start`, own `__cxa_*`): the point is to pin
// down what the ELF interpreter does with these segments, without depending on
// the ziskos runtime. `start.S` mirrors ziskos's `_start` ordering.

#include <stdint.h>

// ---------------------------------------------------------------- ZisK output
// Word 0 of the output area is the word count, followed by the words themselves.
static constexpr uint64_t OUTPUT_ADDR = 0xa0410000ULL;

static uint32_t *const out_n = reinterpret_cast<uint32_t *>(OUTPUT_ADDR);
static uint32_t *const out_w = reinterpret_cast<uint32_t *>(OUTPUT_ADDR + 4);
static uint32_t out_i = 0;  // in .bss: relies on the loader's zero-fill

static void emit(uint32_t v) {
    out_w[out_i++] = v;
    *out_n = out_i;
}

// ------------------------------------------------- minimal __cxa_* machinery
// The same contract ziskos provides (see ziskos/entrypoint/src/lib.rs): the
// compiler registers static destructors with `__cxa_atexit` at construction
// time and expects them to run in reverse order at exit.
extern "C" {
void *__dso_handle = nullptr;

typedef void (*dtor_fn)(void *);
static dtor_fn atexit_fns[16];
static void *atexit_args[16];
static int atexit_len = 0;

int __cxa_atexit(dtor_fn f, void *arg, void *) {
    if (atexit_len >= 16) return -1;
    atexit_fns[atexit_len] = f;
    atexit_args[atexit_len] = arg;
    ++atexit_len;
    return 0;
}

// Called by `start.S` after `main` returns, before `.fini_array`.
void __cxa_finalize_all() {
    while (atexit_len > 0) {
        --atexit_len;
        atexit_fns[atexit_len](atexit_args[atexit_len]);
    }
}

// Single-threaded guest: guard acquire/release need no atomics.
int __cxa_guard_acquire(int64_t *g) { return *g == 0; }
void __cxa_guard_release(int64_t *g) { *g = 1; }
void __cxa_guard_abort(int64_t *) {}
}

// The deleting destructor of a polymorphic class references `operator delete`
// even when it is never called; freestanding, so provide it here. Nothing in
// this test allocates, so `operator new` just traps.
void *operator new(unsigned long) {
    for (;;) {
    }
}
void operator delete(void *) {}
void operator delete(void *, unsigned long) {}

// ------------------------------------------------------------- test subjects

// 1. Plain global with a non-trivial constructor *and* destructor.
struct Counter {
    uint32_t v;
    Counter(uint32_t start) : v(start) { emit(0xC0000000u | start); }
    ~Counter() { emit(0xD0000000u | v); }
};
Counter g_counter(0x11);

// 2. Prioritised constructors: `SORT_BY_INIT_PRIORITY` in the linker script must
//    order these ahead of the unprioritised ones.
struct Marker {
    Marker(uint32_t id) { emit(0xA0000000u | id); }
};
Marker __attribute__((init_priority(101))) g_first(1);
Marker __attribute__((init_priority(102))) g_second(2);

// 3. Dynamically initialised global: the value is not a link-time constant, so
//    it can only be right if the initializer actually ran.
static uint32_t seed() { return 0x1234u ^ 0x5678u; }
uint32_t g_dynamic = seed();

// 4. Virtual dispatch through a vtable held in ROM.
struct Base {
    virtual uint32_t tag() const { return 0xBA5E; }
    virtual ~Base() {}
};
struct Derived : Base {
    uint32_t tag() const override { return 0xDE81; }
};
Derived g_derived;
Base *g_base_ptr = &g_derived;  // relocated pointer in .data

// 5. Constructor that writes through pointers into another global.
struct Node {
    uint32_t id;
    Node *next;
};
Node g_nodes[3] = {{1, nullptr}, {2, nullptr}, {3, nullptr}};
struct Linker {
    Linker() {
        g_nodes[0].next = &g_nodes[1];
        g_nodes[1].next = &g_nodes[2];
        emit(0xE0000000u);
    }
};
Linker g_linker;

// 6. Function-local static: lazily constructed behind a `.bss` guard variable.
static uint32_t lazy() {
    static Counter local(0x22);
    return local.v;
}

// 7. Read-only table read from a constructor (a ROM load during static init).
static const uint32_t k_table[4] = {0xAA, 0xBB, 0xCC, 0xDD};
struct TableSum {
    uint32_t sum = 0;
    TableSum() {
        for (uint32_t i = 0; i < 4; ++i) sum += k_table[i];
        emit(0xF0000000u | sum);
    }
};
TableSum g_sum;

// --------------------------------------------------------------------- main
extern "C" int main() {
    emit(0x00000001u);                 // main reached
    emit(g_counter.v);                 // 0x11    -> ctor ran
    emit(g_dynamic);                   // 0x444c  -> dynamic initializer ran
    emit(g_base_ptr->tag());           // 0xde81  -> vtable in ROM is usable
    emit(g_nodes[0].next->next->id);   // 3       -> ctor linked the nodes
    emit(lazy());                      // 0x22    -> guarded local static
    emit(g_sum.sum);                   // 0x30e   -> rodata read from a ctor
    emit(0x0000FFFFu);                 // main done
    return 0;
}
