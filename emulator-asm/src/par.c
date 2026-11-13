#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <fcntl.h>
#include <semaphore.h>
#include <signal.h>
#include <time.h>
#include <unistd.h>
#include <dirent.h>
#include <ctype.h>
#include <sched.h>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/wait.h>
#include "par.hpp"
#include "par_events.hpp"
#include "asm_provided.hpp"
#include "constants.hpp"
#include "globals.hpp"
#include "server.hpp"
#include "trace.hpp"
#include "emu.hpp"
#include "log.hpp"
#include "../../lib-c/c/src/fcall/fcall.hpp"

/*
 * Fork-join regions in the minimal trace (MT) and in the memory operations (MO).
 *
 * A region (ziskasm/zisklib/zkvm/parallel.zisk) runs its tasks as if one after another, and every
 * pass through its task loop starts at a BOUNDARY marker, on a chunk boundary, in the same state
 * but for s11, the task index. So the chunk that starts at the first pass is, but for its step and
 * s11 in MT, the first chunk of every pass, and any task can run from it. An MO chunk holds no
 * state at all: its header is its end flag and its size, and its records count steps from the
 * chunk's start, so it is the same wherever it lands.
 *
 * Every chunk reaches the reader as an event (par_events.hpp). Outside regions run apart, the
 * process that owns the main trace publishes each chunk as it closes it (MAIN). At a region's
 * first pass the parent saves that chunk's prefix (its header and the reads, or the records, the
 * BOUNDARY step made before calling here), publishes the region (REGION), and forks one process
 * per task. Each task's process writes its trace to its own trace object, mapped over the trace at
 * the same addresses, starting with the prefix, and publishes each of its chunks as it closes it
 * (TASK_CHUNK), so the reader counts and plans them while the tasks run, and nothing is copied. In
 * MT a task's chunks say at which step they start as if the task ran first, from the region's
 * first step; the reader adds the steps of the tasks before it. A task's process sees the guest
 * memory as it was, privately, but for the memory the task owns (mem_start..mem_end), which it
 * shares with the parent. At its next pass it checks that the pass starts in the first pass's
 * state (in MO only the BOUNDARY step's records: the MT emulator runs the same program and checks
 * the rest), publishes how long the task ran (TASK_DONE) and exits. Once every task's process has
 * exited, the parent opens the region's last pass where the first pass's chunk was, publishes the
 * region's end (REGION_END), and leaves the emulation at the last pass: nothing of the region is
 * in the main trace.
 *
 * The tasks' contract (ZkvmTask in ziskos/entrypoint/src/zisklib/lib/zkvm_parallel.rs, and the
 * book's Parallel Execution page) is what makes this work: a task's process keeps only its writes to
 * the memory it owns, sees host state (streamed input, precompile results, randomness) where the
 * region started, and must not end the program; a guest that ends or faults inside a task aborts the
 * run with PAR_ABORT_TASK_ENDED or PAR_ABORT_TASK_FAULTED. A failure of the host's inside a task
 * (no room in /dev/shm for its trace, max_steps, a reset) aborts it with PAR_ABORT_ERROR, as any
 * other failure of the region does, so that it is never taken for the guest's. When this process
 * cannot run a region's tasks apart (see _par_fork and _par_boundary for when), it runs them one
 * after another, as any other code, publishing MAIN chunks.
 */

// The register that holds the task index (PAR_TASK_INDEX_REG in core/src/fcall.rs), and its word
// in a minimal-trace chunk header
#define PAR_TASK_INDEX_REG 27
#define PAR_CHUNK_TASK_INDEX MT_CHUNK_REG_INDEX(PAR_TASK_INDEX_REG)

#define PAR_PAGE_SIZE 4096ULL

// The most tasks one run runs apart, per service. The reader maps every task's trace object over
// the whole trace range (PAR_TASK_TRACE_SIZE of address space each, MT's and MO's in one process),
// which bounds how many it can hold; a region past the budget runs one task after another
#define PAR_MAX_TASKS_PER_RUN 512ULL

// Provided by the generated assembly; the fcall context is a struct FcallContext (see emu.c)
extern uint64_t fcall_ctx;
extern uint64_t reg_27;

// A task as the region's driver reads it: ZkvmTask in ziskos/entrypoint/src/zisklib/lib/
// zkvm_parallel.rs, zkvm_task_t in ziskasm/lang/c/include/zkvm_parallel.h
typedef struct {
    uint64_t entry;
    uint64_t arg;
    uint64_t out;
    uint64_t stack_top;
    uint64_t mem_start;
    uint64_t mem_end;
} ZkvmTask;

// What a task's process leaves for its parent, in memory they share, before it exits
typedef struct {
    uint64_t steps;     // Steps it ran, from the region's first step to its next pass
    uint64_t chunks;    // Chunks it published
    uint64_t bytes;     // Bytes of trace they take
    volatile uint64_t done; // 1 once the above are written
} ParTaskRecord;

// The open region
static struct {
    bool active;       // From its FORK to its last pass
    bool parallel;     // Its tasks run apart
    uint64_t tasks;    // Guest address of its ZkvmTask array
    uint64_t n;        // Number of tasks
    ZkvmTask * list;   // Run apart, its tasks as _par_fork checked them, in this process's memory
} region;

// The first pass's chunk of the region being run apart: where it is, and its prefix, its header
// and `reads` reads (MT) or records (MO)
static uint64_t region_prefix_address = 0;
static uint8_t * region_prefix = NULL;
static uint64_t region_prefix_reads = 0;

// Regions run apart in this run, which number their task trace objects
static uint64_t regions_run = 0;

// The region run apart whose tasks run, which a failure is reported in, or PAR_NONE
static uint64_t current_region = PAR_NONE;

// The chunk events object, mapped in MT and MO
static ParEvents * events = NULL;

// The task trace objects created in this run, which the next run deletes
static char ** task_objects = NULL;
static uint64_t task_objects_count = 0;
static uint64_t task_objects_capacity = 0;

// This process, when it runs a task for its parent
static bool is_child = false;
static uint64_t child_index = 0;
static int child_trace_fd = -1;           // Its task trace object
static uint64_t child_prefix_address = 0; // Where its chunks start
static uint64_t child_first_step = 0;     // The step the region's first pass starts at
static uint64_t child_chunk_address = 0;  // Where the chunk it writes starts
static uint64_t child_chunks = 0;         // Chunks it published
static ParTaskRecord * child_record = NULL;

// Words of a chunk's header in this generation method; its last one is the chunk's reads (MT) or
// records (MO) size
static uint64_t par_chunk_header_words (void)
{
    return (gen_method == MemOp) ? MO_CHUNK_HEADER_WORDS : MT_CHUNK_HEADER_WORDS;
}

// The service's name in its object names: MT or MO
static const char * par_service_name (void)
{
    return (gen_method == MemOp) ? "MO" : "MT";
}

/**********/
/* CONFIG */
/**********/

// CPUs a cgroup v2 cpu.max file allows, "<quota> <period>": quota / period rounded up, or 0 if it
// sets no limit ("max <period>") or cannot be read
static uint64_t par_cpu_max_cpus (const char * file_name)
{
    FILE * file = fopen(file_name, "r");
    if (file == NULL) return 0;
    char quota_text[32];
    unsigned long long period;
    int fields = fscanf(file, "%31s %llu", quota_text, &period);
    fclose(file);
    if ((fields != 2) || (period == 0) || !isdigit((unsigned char)quota_text[0])) return 0;
    char * end;
    errno = 0;
    unsigned long long quota = strtoull(quota_text, &end, 10);
    if ((errno != 0) || (*end != 0) || (quota == 0)) return 0;
    return quota / period + ((quota % period) != 0);
}

// CPUs the cgroup v2 CPU quotas of this process's cgroup and of its ancestors allow, the fewest of
// them, or 0 if none sets a limit or none can be read
static uint64_t par_cgroup_cpus (void)
{
    FILE * file = fopen("/proc/self/cgroup", "r");
    if (file == NULL) return 0;
    char line[4096];
    bool found = false;
    while (fgets(line, sizeof(line), file) != NULL)
    {
        if (strncmp(line, "0::", 3) == 0) { found = true; break; }
    }
    fclose(file);
    if (!found) return 0;

    // The cgroup's path, "/" for the root, without its newline or a trailing slash
    const char * path = line + 3;
    size_t length = strcspn(path, "\n");
    while ((length > 0) && (path[length - 1] == '/')) length--;

    uint64_t cpus = 0;
    while (true)
    {
        char file_name[sizeof(line) + 32];
        snprintf(file_name, sizeof(file_name), "/sys/fs/cgroup%.*s/cpu.max", (int)length, path);
        uint64_t allowed = par_cpu_max_cpus(file_name);
        if ((allowed > 0) && ((cpus == 0) || (allowed < cpus))) cpus = allowed;
        if (length == 0) break;
        // Up to the parent cgroup
        while ((length > 0) && (path[length - 1] != '/')) length--;
        if (length > 0) length--;
    }
    return cpus;
}

// CPUs this process may run on: those in its affinity mask, or the online CPUs if it cannot be
// read, and no more than its cgroup's CPU quota allows; at least 1
static uint64_t par_available_cpus (void)
{
    uint64_t cpus = 0;
    cpu_set_t set;
    if (sched_getaffinity(0, sizeof(set), &set) == 0)
    {
        cpus = (uint64_t)CPU_COUNT(&set);
    }
    if (cpus == 0)
    {
        long online = sysconf(_SC_NPROCESSORS_ONLN);
        cpus = (online > 0) ? (uint64_t)online : 1;
    }
    uint64_t quota = par_cgroup_cpus();
    if ((quota > 0) && (quota < cpus)) cpus = quota;
    return cpus;
}

// Processes running tasks at the same time: ZISK_ASM_PAR_WORKERS, or the CPUs this process may run
// on (par_available_cpus), so that a container or a CPU mask does not run more tasks than CPUs
static uint64_t par_workers (void)
{
    static int64_t workers = -1;
    if (workers < 0)
    {
        const char * value = getenv("ZISK_ASM_PAR_WORKERS");
        if ((value != NULL) && (value[0] != 0))
        {
            char * end;
            long long parsed = strtoll(value, &end, 10);
            workers = ((*end == 0) && (parsed > 0)) ? parsed : 1;
        }
        else
        {
            workers = (int64_t)par_available_cpus();
            if (verbose) asm_printf("par_workers() %ld, the CPUs this process may run on (ZISK_ASM_PAR_WORKERS unset)\n", workers);
        }
    }
    return (uint64_t)workers;
}

static int par_compare_ranges (const void * a, const void * b)
{
    uint64_t x = ((const uint64_t *)a)[0];
    uint64_t y = ((const uint64_t *)b)[0];
    return (x > y) - (x < y);
}

// Whether no two of the n tasks own the same memory: each task's writes to its range reach the
// parent, so two tasks owning one page would race
static bool par_ranges_disjoint (const ZkvmTask * list, uint64_t n)
{
    uint64_t (*ranges)[2] = malloc(n * sizeof(*ranges));
    if (ranges == NULL) return false;
    for (uint64_t i = 0; i < n; i++)
    {
        ranges[i][0] = list[i].mem_start;
        ranges[i][1] = list[i].mem_end;
    }
    qsort(ranges, n, sizeof(*ranges), par_compare_ranges);
    bool disjoint = true;
    for (uint64_t i = 1; disjoint && (i < n); i++) disjoint = (ranges[i - 1][1] <= ranges[i][0]);
    free(ranges);
    return disjoint;
}

// The guest's task list, copied to this process's memory, if it is what the driver lays out: n
// tasks, in the RAM, each owning a page-aligned range of the RAM that holds its stack, no two of
// them the same memory and none of them the list's, within the run's budget of tasks run apart.
// The guest gives it, so it is checked before anything reads through it or forks, and the tasks'
// processes set up from the copy checked, not from the guest's list, which a task could change
// while others wait to be forked. NULL, having said why and set `reason` (PAR_SEQUENTIAL_*), if it
// is not; the region then runs one task after another, which is how the driver runs it anyway
static ZkvmTask * par_tasks_copy (uint64_t tasks, uint64_t n, uint64_t * reason)
{
    const uint64_t ram_end = RAM_ADDR + RAM_SIZE;
    if (n > PAR_MAX_TASKS_PER_RUN - task_objects_count)
    {
        asm_printf("WARNING: fork-join region of %lu tasks would take this run past %llu tasks run apart; it runs them one after another\n", n, PAR_MAX_TASKS_PER_RUN);
        *reason = PAR_SEQUENTIAL_BUDGET;
        return NULL;
    }
    const uint64_t bytes = n * sizeof(ZkvmTask);
    if (((tasks % 8) != 0) || (tasks < RAM_ADDR) || (tasks > ram_end) || (bytes > ram_end - tasks))
    {
        asm_printf("WARNING: fork-join region's task list 0x%lx of %lu tasks is not in the RAM; it runs them one after another\n", tasks, n);
        *reason = PAR_SEQUENTIAL_TASK_LIST;
        return NULL;
    }
    ZkvmTask * list = malloc(bytes);
    if (list == NULL)
    {
        asm_printf("WARNING: fork-join region of %lu tasks failed copying its task list; it runs them one after another\n", n);
        *reason = PAR_SEQUENTIAL_HOST;
        return NULL;
    }
    memcpy(list, (const void *)tasks, bytes);
    for (uint64_t i = 0; i < n; i++)
    {
        const ZkvmTask * task = &list[i];
        if ((((task->mem_start | task->mem_end) & (PAR_PAGE_SIZE - 1)) != 0) ||
            (task->mem_start < RAM_ADDR) || (task->mem_end > ram_end) ||
            (task->mem_start > task->stack_top) || (task->stack_top > task->mem_end))
        {
            asm_printf("WARNING: fork-join task %lu owns 0x%lx..0x%lx with its stack at 0x%lx, not page-aligned RAM that holds its stack; the region runs its tasks one after another\n",
                i, task->mem_start, task->mem_end, task->stack_top);
            free(list);
            *reason = PAR_SEQUENTIAL_TASK_LIST;
            return NULL;
        }
        // The driver reads each task's entry, argument and stack from the guest's list, in the
        // task's process: a task that owned it could change what the tasks after it run
        if ((task->mem_start < task->mem_end) && (task->mem_start < tasks + bytes) && (tasks < task->mem_end))
        {
            asm_printf("WARNING: fork-join task %lu owns 0x%lx..0x%lx, which holds the region's task list at 0x%lx; the region runs its tasks one after another\n",
                i, task->mem_start, task->mem_end, tasks);
            free(list);
            *reason = PAR_SEQUENTIAL_TASK_LIST;
            return NULL;
        }
    }
    if (!par_ranges_disjoint(list, n))
    {
        asm_printf("WARNING: fork-join region's tasks own overlapping memory; it runs them one after another\n");
        free(list);
        *reason = PAR_SEQUENTIAL_TASK_LIST;
        return NULL;
    }
    return list;
}

/**********/
/* EVENTS */
/**********/

// Empties the slots the last run filled and starts the next run's events at slot 0. A reader
// waits on a slot whose kind is still 0, so a slot it is due to read must never hold an older
// event, from this service or from another one that used the object before it
static void par_events_clear (void)
{
    uint64_t used = __atomic_load_n(&events->tail, __ATOMIC_ACQUIRE);
    if (used > PAR_EVENTS_CAPACITY) used = PAR_EVENTS_CAPACITY;
    memset(events->events, 0, used * sizeof(ParEvent));
    __atomic_store_n(&events->tail, 0, __ATOMIC_RELEASE);
    events->regions_sequential = 0;
    events->sequential_reasons = 0;
    events->failure_region = PAR_NONE;
    events->failure_task = PAR_NONE;
    events->failure_detail = 0;
    __atomic_store_n(&events->failure, 0, __ATOMIC_RELEASE);
}

// Reports a region run one task after another, and why (PAR_SEQUENTIAL_*), for the reader. Only
// the process that owns the main trace reports: a task's process never runs a region of its own
static void par_report_sequential (uint64_t reason)
{
    if ((events == NULL) || is_child) return;
    events->regions_sequential++;
    events->sequential_reasons |= (1ULL << reason);
}

// Reports how the run's region failed (PAR_FAILURE_*), in the region run apart if any, for the
// reader to name in its error: the first failure only, which the others follow from
static void par_report_failure (uint64_t failure, uint64_t task, uint64_t detail)
{
    if ((events == NULL) || is_child) return;
    if (__atomic_load_n(&events->failure, __ATOMIC_ACQUIRE) != 0) return;
    events->failure_region = current_region;
    events->failure_task = task;
    events->failure_detail = detail;
    __atomic_store_n(&events->failure, failure, __ATOMIC_RELEASE);
}

// Whether `name` is the name of a task trace object of `prefix` (`<sem_prefix>_<SVC>_r`): the
// prefix, then <region>_t<task>, both decimal
static bool par_is_task_object (const char * name, const char * prefix, size_t prefix_length)
{
    if (strncmp(name, prefix, prefix_length) != 0) return false;
    const char * p = name + prefix_length;
    if (!isdigit((unsigned char)*p)) return false;
    while (isdigit((unsigned char)*p)) p++;
    if ((p[0] != '_') || (p[1] != 't')) return false;
    p += 2;
    if (!isdigit((unsigned char)*p)) return false;
    while (isdigit((unsigned char)*p)) p++;
    return *p == 0;
}

// Unlinks the task trace objects of this program and service that a service which died left
// behind. The reader unlinks each one it maps, and a service those of its last run at its next run,
// but a service that dies leaves the ones nobody mapped, each as large as the trace its task wrote.
// Their names carry the program's prefix (see par_create_task_object), so only this program's are
// matched; and a service is set up between its program's runs, so none of them is in use
static void par_sweep_task_objects (void)
{
    char prefix[MAX_SHM_PREFIX_LENGTH + 16];
    int length = snprintf(prefix, sizeof(prefix), "%s_%s_r", sem_prefix, par_service_name());
    if ((length < 0) || ((size_t)length >= sizeof(prefix))) return;
    DIR * dir = opendir("/dev/shm");
    if (dir == NULL) return;
    uint64_t swept = 0;
    struct dirent * entry;
    while ((entry = readdir(dir)) != NULL)
    {
        if (par_is_task_object(entry->d_name, prefix, (size_t)length) && (shm_unlink(entry->d_name) == 0))
        {
            swept++;
        }
    }
    closedir(dir);
    if (swept > 0)
    {
        asm_printf("WARNING: removed %lu fork-join task trace objects %s* that an earlier service left\n", swept, prefix);
    }
}

void par_events_setup (void)
{
    if ((shmem_events_name[0] == 0) || (!create_output_shm && !open_output_shm))
    {
        return;
    }

    int fd;
    if (create_output_shm)
    {
        // Make sure it is a new one, as the other output objects
        shm_unlink(shmem_events_name);
        fd = shm_open(shmem_events_name, O_RDWR | O_CREAT | O_EXCL, 0666);
        if (fd < 0)
        {
            asm_printf("ERROR: par_events_setup() failed creating shm_open(%s) errno=%d=%s\n", shmem_events_name, errno, strerror(errno));
            exit(-1);
        }
        if (ftruncate(fd, PAR_EVENTS_BYTES) != 0)
        {
            asm_printf("ERROR: par_events_setup() failed calling ftruncate(%s) errno=%d=%s\n", shmem_events_name, errno, strerror(errno));
            exit(-1);
        }
    }
    else
    {
        fd = shm_open(shmem_events_name, O_RDWR, 0666);
        if (fd < 0)
        {
            asm_printf("ERROR: par_events_setup() failed opening shm_open(%s) errno=%d=%s\n", shmem_events_name, errno, strerror(errno));
            exit(-1);
        }
        struct stat object_stat;
        if ((fstat(fd, &object_stat) != 0) || ((uint64_t)object_stat.st_size < PAR_EVENTS_BYTES))
        {
            asm_printf("ERROR: par_events_setup() found %s smaller than %lu B\n", shmem_events_name, (uint64_t)PAR_EVENTS_BYTES);
            exit(-1);
        }
    }

    if (open_output_shm)
    {
        void * mapped = mmap(NULL, PAR_EVENTS_BYTES, PROT_READ | PROT_WRITE, MAP_SHARED | map_locked_flag, fd, 0);
        if (mapped == MAP_FAILED)
        {
            asm_printf("ERROR: par_events_setup() failed calling mmap(%s) errno=%d=%s\n", shmem_events_name, errno, strerror(errno));
            exit(-1);
        }
        events = (ParEvents *)mapped;
        events->version = PAR_EVENTS_VERSION;
        par_events_clear();
        par_sweep_task_objects();
    }
    close(fd);
    if (verbose) asm_printf("par_events_setup() mapped %s, %lu B\n", shmem_events_name, (uint64_t)PAR_EVENTS_BYTES);
}

// Deletes the task trace objects of the last run; the reader keeps whatever it mapped of them
static void par_delete_task_objects (void)
{
    for (uint64_t i = 0; i < task_objects_count; i++)
    {
        shm_unlink(task_objects[i]);
        free(task_objects[i]);
    }
    task_objects_count = 0;
}

void par_events_cleanup (void)
{
    par_delete_task_objects();
    free(task_objects);
    task_objects = NULL;
    task_objects_capacity = 0;
    if (events != NULL)
    {
        munmap(events, PAR_EVENTS_BYTES);
        events = NULL;
    }
    if ((shmem_events_name[0] != 0) && delete_output_shm)
    {
        shm_unlink(shmem_events_name);
    }
}

void par_run_start (void)
{
    par_reset();
    par_delete_task_objects();
    regions_run = 0;
    if (events != NULL)
    {
        par_events_clear();
    }
}

bool par_events_enabled (void)
{
    return events != NULL;
}

// Appends an event and posts it. Returns false, having said why, when it cannot
static bool par_event (uint64_t kind, uint64_t a, uint64_t b, uint64_t c)
{
    uint64_t slot = __atomic_fetch_add(&events->tail, 1, __ATOMIC_ACQ_REL);
    if (slot >= PAR_EVENTS_CAPACITY)
    {
        asm_printf("ERROR: fork-join events reached their capacity of %llu in this run\n", (unsigned long long)PAR_EVENTS_CAPACITY);
        par_report_failure(PAR_FAILURE_EVENTS, PAR_NONE, 0);
        return false;
    }
    ParEvent * event = &events->events[slot];
    event->a = a;
    event->b = b;
    event->c = c;
    __atomic_store_n(&event->kind, kind, __ATOMIC_RELEASE);
    if (call_chunk_done && (sem_post(sem_chunk_done) != 0))
    {
        int error = errno;
        asm_printf("ERROR: fork-join failed calling sem_post(%s) errno=%d=%s\n", sem_chunk_done_name, error, strerror(error));
        par_report_failure(PAR_FAILURE_EVENTS, PAR_NONE, (uint64_t)error);
        return false;
    }
    return true;
}

void par_chunk_done (void)
{
    // A task's chunk, from its trace object; it just closed the one that started where the last one
    // ended, or where the region's first pass's chunk is
    if (is_child)
    {
        uint64_t offset = child_chunk_address - TRACE_ADDR;
        child_chunk_address = MEM_CHUNK_ADDRESS;
        if (!par_event(PAR_EVENT_TASK_CHUNK, child_index, child_chunks, offset))
        {
            par_child_exit(PAR_EXIT_PUBLISH);
        }
        child_chunks++;
        return;
    }

    // The next chunk of the main trace
    if (!par_event(PAR_EVENT_MAIN, 0, 0, 0))
    {
        server_abort_emulation(PAR_ABORT_ERROR);
    }
}

/*********/
/* CHILD */
/*********/

bool par_is_child (void)
{
    return is_child;
}

void par_child_exit (int code)
{
    // Never the server's exit path: the parent answers the request, and nothing of this process
    // but its trace object, its events and its record is of any use
    _exit(code);
}

void par_child_end (bool aborted)
{
    int code;
    const char * why;
    if (aborted)
    {
        code = PAR_EXIT_ABORTED;
        why = "the guest faulted";
    }
    else if (MEM_END != 0)
    {
        code = PAR_EXIT_ENDED;
        why = "the guest ended";
    }
    // The host's limits stop the emulation without the guest's end: a reset, on which the waits
    // for input and precompile results return when the run is cancelled, and max_steps, which
    // the step count reaches at a chunk's end
    else if ((precompile_reset_address != NULL) && (*precompile_reset_address != 0))
    {
        code = PAR_EXIT_RESET;
        why = "a reset stopped the emulation";
    }
    else if (MEM_STEP >= max_steps)
    {
        code = PAR_EXIT_MAX_STEPS;
        why = "the emulation reached max_steps";
    }
    // Anything else that leaves the emulation, such as a jump out of the ROM, is the guest's
    else
    {
        code = PAR_EXIT_ENDED;
        why = "the guest left the emulation";
    }
    asm_printf("ERROR: %s inside fork-join task %lu, near step %lu\n", why, child_index, MEM_STEP);
    par_child_exit(code);
}

// The server's handling of SIGBUS, which a task's process passes on what it does not handle
static struct sigaction child_server_sigbus;

// SIGBUS in a task's process. Its trace object is sparse, and so are the events and the guest
// RAM when they are not locked in memory, so touching a page of any of them that /dev/shm, which
// backs them, has no room left for raises SIGBUS: the host's failure, which the parent must not
// report as the guest faulting inside the task. Any other goes to the server's handling, as in
// the parent. Only async-signal-safe calls
static void par_child_on_sigbus (int sig, siginfo_t * info, void * context)
{
    uint64_t address = (uint64_t)info->si_addr;
    bool in_trace = (address >= TRACE_ADDR) && (address - TRACE_ADDR < PAR_TASK_TRACE_SIZE);
    bool in_events = (events != NULL) && (address >= (uint64_t)events) && (address - (uint64_t)events < PAR_EVENTS_BYTES);
    bool in_ram = (address >= RAM_ADDR) && (address - RAM_ADDR < RAM_SIZE);
    if ((info->si_code > 0) && (in_trace || in_events || in_ram))
    {
        par_child_exit(PAR_EXIT_SHM_FULL);
    }
    if ((child_server_sigbus.sa_flags & SA_SIGINFO) != 0)
    {
        child_server_sigbus.sa_sigaction(sig, info, context);
    }
    else if ((child_server_sigbus.sa_handler != SIG_DFL) && (child_server_sigbus.sa_handler != SIG_IGN))
    {
        child_server_sigbus.sa_handler(sig);
    }
    else
    {
        signal(sig, SIG_DFL);
        raise(sig);
    }
}

void par_child_map_trace (uint64_t address, uint64_t size)
{
    // Its trace object at the same offsets as the trace's addresses, so the reader finds a chunk at
    // its address minus TRACE_ADDR. In the base pages /dev/shm gives: huge pages, where a mount
    // allows them, cost more to allocate and clear than they save, and slow a region down
    void * trace = mmap((void *)address, size, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED, child_trace_fd, (off_t)(address - TRACE_ADDR));
    if (trace == MAP_FAILED)
    {
        asm_printf("ERROR: fork-join task %lu failed mapping its trace at 0x%lx (%lu B) errno=%d=%s\n", child_index, address, size, errno, strerror(errno));
        par_child_exit(PAR_EXIT_SETUP);
    }
}

// Turns this freshly forked process into task `index`'s: its trace goes to its trace object,
// mapped at the same addresses, starting with the region's first chunk prefix; it sees the guest
// memory as it was, privately, but for the memory the task owns, which it shares with the parent
static void par_child_setup (uint64_t index, int trace_fd, pid_t parent, uint64_t prefix_bytes,
                             uint64_t first_step, ParTaskRecord * record)
{
    is_child = true;
    child_index = index;
    child_trace_fd = trace_fd;
    child_prefix_address = region_prefix_address;
    child_first_step = first_step;
    child_chunk_address = region_prefix_address;
    child_chunks = 0;
    child_record = record;

    // Die with the parent, which may have died before this call
    if ((prctl(PR_SET_PDEATHSIG, SIGKILL) != 0) || (getppid() != parent))
    {
        par_child_exit(PAR_EXIT_SETUP);
    }

    // Before its first write to its trace: a fault writing it, an event or the RAM is the host's
    // (see par_child_on_sigbus). Only this process's handling changes, not its parent's
    struct sigaction sigbus = {
        .sa_sigaction = par_child_on_sigbus,
        .sa_flags     = SA_SIGINFO | SA_ONSTACK | SA_NODEFER,
    };
    sigemptyset(&sigbus.sa_mask);
    if (sigaction(SIGBUS, &sigbus, &child_server_sigbus) != 0)
    {
        asm_printf("ERROR: fork-join task %lu failed calling sigaction(SIGBUS) errno=%d=%s\n", index, errno, strerror(errno));
        par_child_exit(PAR_EXIT_SETUP);
    }

    // The trace: its trace object over everything the parent has mapped, so that the assembly
    // keeps writing at the same addresses, and grows it there too (see trace_map_next_chunk)
    par_child_map_trace(TRACE_ADDR, trace_total_mapped_size);
    memcpy((void *)region_prefix_address, region_prefix, prefix_bytes);
    if (gen_method == MinimalTrace)
    {
        ((uint64_t *)region_prefix_address)[PAR_CHUNK_TASK_INDEX] = index;
    }

    // The guest memory: a private copy, but for the memory the task owns (page-aligned RAM, as
    // _par_fork checked in its copy of the task list, which no task can change). The file
    // descriptor was closed after the setup, so open the memory again by name
    int ram_fd = shm_open(shmem_ram_name, O_RDWR, 0666);
    if (ram_fd < 0)
    {
        asm_printf("ERROR: fork-join task %lu failed calling shm_open(%s) errno=%d=%s\n", index, shmem_ram_name, errno, strerror(errno));
        par_child_exit(PAR_EXIT_SETUP);
    }
    void * ram = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_FIXED, ram_fd, 0);
    if (ram == MAP_FAILED)
    {
        asm_printf("ERROR: fork-join task %lu failed mapping its private memory errno=%d=%s\n", index, errno, strerror(errno));
        par_child_exit(PAR_EXIT_SETUP);
    }
    const ZkvmTask * task = &region.list[index];
    if (task->mem_end > task->mem_start)
    {
        void * owned = mmap((void *)task->mem_start, task->mem_end - task->mem_start, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED, ram_fd, (off_t)(task->mem_start - RAM_ADDR));
        if (owned == MAP_FAILED)
        {
            asm_printf("ERROR: fork-join task %lu failed sharing its memory errno=%d=%s\n", index, errno, strerror(errno));
            par_child_exit(PAR_EXIT_SETUP);
        }
    }
    close(ram_fd);

    reg_27 = index;
}

// The name of chunk header word `word`, for messages
static void par_header_word_name (uint64_t word, char * name, size_t size)
{
    if (word == 0) snprintf(name, size, "pc");
    else if (word == 1) snprintf(name, size, "sp");
    else if (word == 2) snprintf(name, size, "c");
    else if (word == MT_CHUNK_STEP_INDEX) snprintf(name, size, "step");
    else snprintf(name, size, "r%lu", word - 3);
}

// The next pass must start in the first pass's state, but for its step and s11, which is what lets
// any task run from the first pass's state. The Rust emulator checks it too, but not when it
// replays chunks, so check it here, where a task that breaks it would make a wrong trace
static void par_child_check_state (uint64_t mem_reads_size)
{
    const uint64_t * expected = (const uint64_t *)region_prefix;
    const uint64_t * found = (const uint64_t *)MEM_CHUNK_ADDRESS;
    bool same = true;

    // The state, which only an MT chunk holds; the MT emulator runs the same program as MO's
    for (uint64_t word = 0; (gen_method == MinimalTrace) && (word < MT_CHUNK_START_WORDS); word++)
    {
        if (word == MT_CHUNK_STEP_INDEX) continue;
        uint64_t want = (word == PAR_CHUNK_TASK_INDEX) ? (child_index + 1) : expected[word];
        if (found[word] != want)
        {
            char name[16];
            par_header_word_name(word, name, sizeof(name));
            asm_printf("ERROR: fork-join task %lu leaves pass %lu with %s = 0x%lx, not 0x%lx\n", child_index, child_index + 1, name, found[word], want);
            same = false;
        }
    }

    // The BOUNDARY step's reads (MT) or records (MO), which the parent puts at the last pass
    if (mem_reads_size != region_prefix_reads)
    {
        asm_printf("ERROR: fork-join task %lu leaves pass %lu with %lu reads before its marker, not %lu\n", child_index, child_index + 1, mem_reads_size, region_prefix_reads);
        same = false;
    }
    else
    {
        for (uint64_t k = 0; k < mem_reads_size; k++)
        {
            uint64_t word = par_chunk_header_words() + k;
            if (found[word] != expected[word])
            {
                asm_printf("ERROR: fork-join task %lu leaves pass %lu with read %lu = 0x%lx, not 0x%lx\n", child_index, child_index + 1, k, found[word], expected[word]);
                same = false;
            }
        }
    }
    if (!same)
    {
        par_child_exit(PAR_EXIT_STATE);
    }
}

// Task's process at its next pass: checks the pass's state, says how long the task ran, and exits
static void par_child_finish (uint64_t mem_reads_size) __attribute__((noreturn));
static void par_child_finish (uint64_t mem_reads_size)
{
    par_child_check_state(mem_reads_size);

    // Every chunk it closed is published, and they fill the steps it ran
    uint64_t steps = MEM_STEP - child_first_step;
    if (((steps % chunk_size) != 0) || (child_chunks != steps / chunk_size) || (child_chunk_address != MEM_CHUNK_ADDRESS))
    {
        asm_printf("ERROR: fork-join task %lu ran %lu steps but published %lu chunks\n", child_index, steps, child_chunks);
        par_child_exit(PAR_EXIT_STATE);
    }
    child_record->steps = steps;
    child_record->chunks = child_chunks;
    child_record->bytes = MEM_CHUNK_ADDRESS - child_prefix_address;
    __sync_synchronize();
    child_record->done = 1;
    if (!par_event(PAR_EVENT_TASK_DONE, child_index, steps, child_chunks))
    {
        par_child_exit(PAR_EXIT_PUBLISH);
    }
    par_child_exit(PAR_EXIT_DONE);
}

/**********/
/* PARENT */
/**********/

// The region being run apart: its tasks' processes and records, for the abort path
static pid_t * region_pids = NULL;
static int * region_fds = NULL; // The tasks' trace objects not yet handed to their processes
static ParTaskRecord * region_records = NULL;
static uint64_t region_records_bytes = 0;

// Stops the region's tasks' processes and frees what the region holds; its task trace objects stay
// until the next run, since the reader may not have opened them yet
static void par_release (void)
{
    if (region_pids != NULL)
    {
        for (uint64_t i = 0; i < region.n; i++)
        {
            if (region_pids[i] > 0)
            {
                kill(region_pids[i], SIGKILL);
                while ((waitpid(region_pids[i], NULL, 0) < 0) && (errno == EINTR)) {}
                region_pids[i] = 0;
            }
        }
        free(region_pids);
        region_pids = NULL;
    }
    if (region_fds != NULL)
    {
        for (uint64_t i = 0; i < region.n; i++)
        {
            if (region_fds[i] >= 0) close(region_fds[i]);
        }
        free(region_fds);
        region_fds = NULL;
    }
    if (region_records != NULL)
    {
        munmap(region_records, region_records_bytes);
        region_records = NULL;
    }
    free(region_prefix);
    region_prefix = NULL;
    free(region.list);
    region.list = NULL;
    region.active = false;
    current_region = PAR_NONE;
}

// Stops the region's tasks and aborts the emulation through the server's recovery path. The main
// trace ends where the region's first pass's chunk is: the parent does not move it while the
// tasks run
static void par_abort_with (int error) __attribute__((noreturn));
static void par_abort_with (int error)
{
    par_release();
    server_abort_emulation(error);
}

static void par_abort (void) __attribute__((noreturn));
static void par_abort (void)
{
    par_abort_with(PAR_ABORT_ERROR);
}

// Reports how the region failed (see par_report_failure) and aborts the run
static void par_fail (uint64_t failure, uint64_t task, uint64_t detail) __attribute__((noreturn));
static void par_fail (uint64_t failure, uint64_t task, uint64_t detail)
{
    par_report_failure(failure, task, detail);
    par_abort();
}

// What a task's process that failed says by its exit code
static const char * par_exit_reason (int code)
{
    switch (code)
    {
        case PAR_EXIT_SETUP: return "could not set up its trace or memory";
        case PAR_EXIT_STATE: return "reached a pass other than the next one, or in another state";
        case PAR_EXIT_PUBLISH: return "could not publish a chunk or its end";
        case PAR_EXIT_SHM_FULL: return "could not write its trace, an event or the guest memory: /dev/shm, which backs them, is full";
        case PAR_EXIT_MAX_STEPS: return "reached max_steps";
        case PAR_EXIT_RESET: return "was stopped by a reset: the run is cancelled";
        default: return "failed";
    }
}

static void par_report_exit (uint64_t index, int status)
{
    if (WIFSIGNALED(status))
        asm_printf("ERROR: fork-join task %lu was killed by signal %d\n", index, WTERMSIG(status));
    else if (WIFEXITED(status) && (WEXITSTATUS(status) != PAR_EXIT_DONE))
        asm_printf("ERROR: fork-join task %lu exited with code %d: it %s\n", index, WEXITSTATUS(status), par_exit_reason(WEXITSTATUS(status)));
    else
        asm_printf("ERROR: fork-join task %lu exited without saying how long it ran\n", index);
}

// Remembers a task trace object, for the next run to delete
static void par_remember_task_object (const char * name)
{
    if (task_objects_count == task_objects_capacity)
    {
        uint64_t capacity = (task_objects_capacity == 0) ? 64 : 2 * task_objects_capacity;
        char ** grown = realloc(task_objects, capacity * sizeof(char *));
        if (grown == NULL)
        {
            asm_printf("ERROR: fork-join failed growing its list of task trace objects\n");
            shm_unlink(name);
            par_fail(PAR_FAILURE_HOST, PAR_NONE, ENOMEM);
        }
        task_objects = grown;
        task_objects_capacity = capacity;
    }
    task_objects[task_objects_count] = strdup(name);
    if (task_objects[task_objects_count] == NULL)
    {
        asm_printf("ERROR: fork-join failed remembering task trace object %s\n", name);
        shm_unlink(name);
        par_fail(PAR_FAILURE_HOST, PAR_NONE, ENOMEM);
    }
    task_objects_count++;
}

// Creates task `index`'s trace object: a new one, as large as the trace can grow, allocated only
// where written. Every task's exists before the region is published, so the reader can map them.
// It is named by the program's prefix, the one its semaphores carry, which its reader knows too,
// not by the segments' one: every program set up on a rank shares that, and a service unlinks its
// last run's objects by name at its next run and at its teardown, which may come while another
// program runs a job (a teardown holds no claim on the segments)
static int par_create_task_object (uint64_t region_index, uint64_t index)
{
    char name[128];
    int length = snprintf(name, sizeof(name), "%s_%s_r%lu_t%lu", sem_prefix, par_service_name(), region_index, index);
    if ((length < 0) || ((size_t)length >= sizeof(name)))
    {
        asm_printf("ERROR: fork-join task %lu's trace object name is too long\n", index);
        par_fail(PAR_FAILURE_HOST, index, ENAMETOOLONG);
    }
    shm_unlink(name);
    int fd = shm_open(name, O_RDWR | O_CREAT | O_EXCL, 0666);
    if (fd < 0)
    {
        int error = errno;
        asm_printf("ERROR: fork-join task %lu failed creating shm_open(%s) errno=%d=%s\n", index, name, error, strerror(error));
        par_fail(PAR_FAILURE_HOST, index, (uint64_t)error);
    }
    par_remember_task_object(name);
    if (ftruncate(fd, PAR_TASK_TRACE_SIZE) != 0)
    {
        int error = errno;
        asm_printf("ERROR: fork-join task %lu failed calling ftruncate(%s) errno=%d=%s\n", index, name, error, strerror(error));
        close(fd);
        par_fail(PAR_FAILURE_HOST, index, (uint64_t)error);
    }
    return fd;
}

// Forks task `index`'s process, which writes to the trace object created for it. Returns true in
// that process, once it is set up
static bool par_spawn (uint64_t index, uint64_t prefix_bytes, uint64_t first_step)
{
    int fd = region_fds[index];
    pid_t parent = getpid();
    pid_t pid = fork();
    if (pid < 0)
    {
        int error = errno;
        asm_printf("ERROR: fork-join task %lu failed calling fork() errno=%d=%s\n", index, error, strerror(error));
        par_fail(PAR_FAILURE_HOST, index, (uint64_t)error);
    }
    if (pid == 0)
    {
        // Only its own trace object
        for (uint64_t i = 0; i < region.n; i++)
        {
            if ((i != index) && (region_fds[i] >= 0)) close(region_fds[i]);
        }
        par_child_setup(index, fd, parent, prefix_bytes, first_step, &region_records[index]);
        return true;
    }
    close(fd);
    region_fds[index] = -1;
    region_pids[index] = pid;
    return false;
}

// What a region's tasks' processes used, as wait4() reports it for each: their CPU time and page
// faults, summed, and the largest of their peak resident sets. A peak counts the pages a process
// shares with the parent, those it inherits at the fork too: an upper bound of what a task adds
typedef struct {
    uint64_t user_us;
    uint64_t system_us;
    uint64_t max_rss_kb;
    uint64_t minor_faults;
    uint64_t major_faults;
} ParTasksUsage;

static uint64_t par_timeval_us (struct timeval time)
{
    return (uint64_t)time.tv_sec * 1000000 + (uint64_t)time.tv_usec;
}

// Adds what a task's process used to its region's; ru_maxrss is in KB on Linux
static void par_add_task_usage (ParTasksUsage * total, const struct rusage * usage)
{
    total->user_us += par_timeval_us(usage->ru_utime);
    total->system_us += par_timeval_us(usage->ru_stime);
    if ((uint64_t)usage->ru_maxrss > total->max_rss_kb) total->max_rss_kb = (uint64_t)usage->ru_maxrss;
    total->minor_faults += (uint64_t)usage->ru_minflt;
    total->major_faults += (uint64_t)usage->ru_majflt;
}

// Runs the open region from its first pass, whose chunk starts at MEM_CHUNK_ADDRESS with
// `mem_reads_size` reads written, and leaves the emulation at its last pass. In a task's process
// it returns as soon as the process is set up, and the assembly runs the task
static void par_run_region (uint64_t mem_reads_size)
{
    uint64_t n = region.n;
    uint64_t workers = par_workers();
    if (workers > n) workers = n;

    uint64_t header_words = par_chunk_header_words();
    uint64_t prefix_bytes = (header_words + mem_reads_size) * 8;
    uint64_t first_step = MEM_STEP;
    region_prefix_address = MEM_CHUNK_ADDRESS;

    struct timeval start, stop;
    uint64_t fork_us = 0, wait_us = 0;
    ParTasksUsage usage = {0};

    // Its tasks see the prefix as the parent leaves it here
    region_prefix = malloc(prefix_bytes);
    region_prefix_reads = mem_reads_size;
    region_pids = calloc(n, sizeof(pid_t));
    region_records_bytes = n * sizeof(ParTaskRecord);
    region_records = mmap(NULL, region_records_bytes, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if ((region_prefix == NULL) || (region_pids == NULL) || (region_records == MAP_FAILED))
    {
        if (region_records == MAP_FAILED) region_records = NULL;
        asm_printf("ERROR: fork-join region of %lu tasks failed allocating its state\n", n);
        par_fail(PAR_FAILURE_HOST, PAR_NONE, ENOMEM);
    }
    memcpy(region_prefix, (void *)region_prefix_address, prefix_bytes);

    // Every task's trace object, before the region is published
    uint64_t region_index = regions_run++;
    current_region = region_index;
    region_fds = malloc(n * sizeof(int));
    if (region_fds == NULL)
    {
        asm_printf("ERROR: fork-join region of %lu tasks failed allocating its state\n", n);
        par_fail(PAR_FAILURE_HOST, PAR_NONE, ENOMEM);
    }
    for (uint64_t i = 0; i < n; i++) region_fds[i] = -1;
    for (uint64_t i = 0; i < n; i++) region_fds[i] = par_create_task_object(region_index, i);

    if (!par_event(PAR_EVENT_REGION, region_index, n, first_step))
    {
        par_abort();
    }

    // A task's process inherits what stdio holds: leave nothing buffered for it to repeat
    fflush(NULL);

    gettimeofday(&start, NULL);
    for (uint64_t i = 0; i < workers; i++)
    {
        if (par_spawn(i, prefix_bytes, first_step)) return;
    }
    gettimeofday(&stop, NULL);
    fork_us += TimeDiff(start, stop);

    // The tasks, as they end, in any order: each publishes its own chunks, so the next one of the
    // window starts as soon as any is done
    uint64_t next = workers;
    uint64_t done = 0;
    uint64_t steps = 0, chunks = 0, bytes = 0;
    while (done < n)
    {
        gettimeofday(&start, NULL);
        int status = 0;
        struct rusage task_usage;
        pid_t pid;
        do
        {
            pid = wait4(-1, &status, 0, &task_usage);
        } while ((pid < 0) && (errno == EINTR));
        gettimeofday(&stop, NULL);
        wait_us += TimeDiff(start, stop);
        if (pid < 0)
        {
            int error = errno;
            asm_printf("ERROR: fork-join region failed calling wait4() errno=%d=%s\n", error, strerror(error));
            par_fail(PAR_FAILURE_HOST, PAR_NONE, (uint64_t)error);
        }
        uint64_t i = 0;
        while ((i < n) && (region_pids[i] != pid)) i++;
        if (i == n)
        {
            // Not a task's process: nothing of the region's
            continue;
        }
        region_pids[i] = 0;
        par_add_task_usage(&usage, &task_usage);

        ParTaskRecord * record = &region_records[i];
        // The guest ended or faulted inside the task: the run aborts, whatever its sequential
        // execution would do there, with an error that says so
        if (WIFEXITED(status) && ((WEXITSTATUS(status) == PAR_EXIT_ENDED) || (WEXITSTATUS(status) == PAR_EXIT_ABORTED)))
        {
            bool ended = (WEXITSTATUS(status) == PAR_EXIT_ENDED);
            asm_printf("ERROR: the guest %s inside task %lu of fork-join region %lu: a task must not end the program. With ZISK_ASM_PAR_WORKERS=1 the region runs one task after another, as in a sequential execution\n",
                ended ? "ended (it exited or panicked)" : "faulted", i, region_index);
            par_report_failure(PAR_FAILURE_TASK_EXIT, i, (uint64_t)WEXITSTATUS(status));
            par_abort_with(ended ? PAR_ABORT_TASK_ENDED : PAR_ABORT_TASK_FAULTED);
        }
        // Any other failure, the host's inside the task included, fails the region
        if (!WIFEXITED(status) || (WEXITSTATUS(status) != PAR_EXIT_DONE) || (record->done != 1))
        {
            par_report_exit(i, status);
            if (WIFSIGNALED(status)) par_fail(PAR_FAILURE_TASK_SIGNAL, i, (uint64_t)WTERMSIG(status));
            if (WIFEXITED(status) && (WEXITSTATUS(status) != PAR_EXIT_DONE)) par_fail(PAR_FAILURE_TASK_EXIT, i, (uint64_t)WEXITSTATUS(status));
            par_fail(PAR_FAILURE_TASK_RECORD, i, 0);
        }
        if ((record->steps == 0) || ((record->steps % chunk_size) != 0) || (record->chunks != record->steps / chunk_size))
        {
            asm_printf("ERROR: fork-join task %lu ran %lu steps into %lu chunks\n", i, record->steps, record->chunks);
            par_fail(PAR_FAILURE_TASK_RECORD, i, 0);
        }
        steps += record->steps;
        chunks += record->chunks;
        bytes += record->bytes;
        done++;

        // The sequential execution would stop at max_steps, inside the region: no way to resume
        // from here, where its last pass starts past it
        if (first_step + steps >= max_steps)
        {
            asm_printf("ERROR: fork-join region reaches step %lu with task %lu, past max_steps = %lu\n", first_step + steps, i, max_steps);
            par_fail(PAR_FAILURE_MAX_STEPS, i, first_step + steps);
        }

        // The next task of the window
        if (next < n)
        {
            gettimeofday(&start, NULL);
            if (par_spawn(next, prefix_bytes, first_step)) return;
            gettimeofday(&stop, NULL);
            fork_us += TimeDiff(start, stop);
            next++;
        }
    }
    uint64_t step = first_step + steps;
    *(uint64_t *)MEM_TRACE_ADDRESS += chunks;

    // The last pass, which leaves the loop, starts with the same chunk, where the first pass's was;
    // in MT, at its own step and with its own task index
    memcpy((void *)region_prefix_address, region_prefix, prefix_bytes);
    if (gen_method == MinimalTrace)
    {
        ((uint64_t *)region_prefix_address)[MT_CHUNK_STEP_INDEX] = step;
        ((uint64_t *)region_prefix_address)[PAR_CHUNK_TASK_INDEX] = n;
    }
    MEM_CHUNK_ADDRESS = region_prefix_address;
    MEM_STEP = step;
    MEM_CHUNK_START_STEP = step;
    reg_27 = n;

    par_release();
    if (!par_event(PAR_EVENT_REGION_END, region_index, 0, 0))
    {
        server_abort_emulation(PAR_ABORT_ERROR);
    }

    if (verbose || metrics)
    {
        asm_printf("fork-join region %lu (%s): %lu tasks on %lu workers, %lu chunks, %lu steps, %lu MB of task traces; fork %lu us, wait for tasks %lu us; tasks' CPU user %lu us, system %lu us, max RSS %lu MB, page faults %lu minor, %lu major\n",
            region_index, par_service_name(), n, workers, chunks, steps, bytes >> 20, fork_us, wait_us,
            usage.user_us, usage.system_us, usage.max_rss_kb >> 10, usage.minor_faults, usage.major_faults);
    }
}

/*************/
/* ASM CALLS */
/*************/

void par_reset (void)
{
    if (!is_child)
    {
        par_release();
    }
}

void _par_fork (void)
{
    uint64_t step = MEM_STEP;
    if (region.active)
    {
        asm_printf("ERROR: fork-join region starts inside another, near step %lu\n", step);
        if (is_child) par_child_exit(PAR_EXIT_STATE);
        par_fail(PAR_FAILURE_PROTOCOL, PAR_NONE, 0);
    }
    const struct FcallContext * fcall = (const struct FcallContext *)&fcall_ctx;
    if (fcall->params_size != 2)
    {
        asm_printf("ERROR: fork-join region near step %lu takes 2 parameters, not %lu\n", step, fcall->params_size);
        if (is_child) par_child_exit(PAR_EXIT_STATE);
        par_fail(PAR_FAILURE_PROTOCOL, PAR_NONE, 0);
    }

    region.active = true;
    region.tasks = fcall->params[0];
    region.n = fcall->params[1];

    // Why it runs one task after another, if it does: a reason of the execution's, said only in
    // verbose mode, or one of the guest's or the run's budget, which par_tasks_copy warns of
    const char * sequential = NULL;
    uint64_t reason = 0;
    if ((gen_method != MinimalTrace) && (gen_method != MemOp)) sequential = "neither a minimal trace nor memory operations";
    else if (events == NULL) sequential = "no chunk events object";
    else if (chunk_size != CHUNK_SIZE) { sequential = "the chunk size is not the one the driver pads to"; reason = PAR_SEQUENTIAL_CHUNK_SIZE; }
    else if (precompile_results_enabled) { sequential = "precompile results"; reason = PAR_SEQUENTIAL_HINTS; }
    else if (par_workers() <= 1) { sequential = "one worker (ZISK_ASM_PAR_WORKERS <= 1, or one CPU to run on)"; reason = PAR_SEQUENTIAL_WORKERS; }
    else if (region.n <= 1) { sequential = "fewer than 2 tasks"; reason = PAR_SEQUENTIAL_FEW_TASKS; }
    region.list = (sequential == NULL) ? par_tasks_copy(region.tasks, region.n, &reason) : NULL;
    region.parallel = (region.list != NULL);
    if (!region.parallel && (reason != 0))
    {
        par_report_sequential(reason);
    }

    if ((verbose || metrics) && (sequential != NULL))
    {
        asm_printf("fork-join region of %lu tasks runs them one after another: %s\n", region.n, sequential);
    }
}

void _par_boundary (uint64_t mem_reads_size, uint64_t step_count_down)
{
    uint64_t index = reg_27;

    // A task's process is done when its task is: at the next pass
    if (is_child)
    {
        if (index != child_index + 1)
        {
            asm_printf("ERROR: fork-join task %lu reached pass %lu\n", child_index, index);
            par_child_exit(PAR_EXIT_STATE);
        }
        par_child_finish(mem_reads_size);
    }

    if (!region.active)
    {
        asm_printf("ERROR: fork-join pass %lu starts outside a region\n", index);
        par_fail(PAR_FAILURE_PROTOCOL, PAR_NONE, 0);
    }

    // Run apart, a region's first pass must start a chunk, which every task's chunks then start
    // from; if it does not, nothing is forked yet, so it runs one task after another instead
    if (region.parallel && (index == 0) && ((step_count_down != chunk_size) || ((MEM_STEP % chunk_size) != 0)))
    {
        asm_printf("WARNING: fork-join region's first pass at step %lu does not start a chunk (count down %lu of %lu); it runs its tasks one after another\n",
            MEM_STEP + chunk_size - step_count_down, step_count_down, chunk_size);
        region.parallel = false;
        free(region.list);
        region.list = NULL;
        par_report_sequential(PAR_SEQUENTIAL_FIRST_PASS);
    }

    // Run one after another, the region ends at its last pass
    if (!region.parallel)
    {
        if (index == region.n)
        {
            region.active = false;
        }
        return;
    }

    // Run apart, only the first pass gets here: the parent comes back at the last one
    if (index != 0)
    {
        asm_printf("ERROR: fork-join region run apart reached pass %lu\n", index);
        par_fail(PAR_FAILURE_PROTOCOL, PAR_NONE, 0);
    }
    par_run_region(mem_reads_size);
}
