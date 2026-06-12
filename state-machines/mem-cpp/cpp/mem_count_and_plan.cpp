#include <memory>
#include <algorithm>
#include "api.hpp"
#include "tools.hpp"
#include "mem_count_and_plan.hpp"
#include "mem_stats.hpp"
#include "instance_meta.hpp"

static void mkdir_recursive(const char *path) {
    char tmp[512];
    snprintf(tmp, sizeof(tmp), "%s", path);
    for (char *p = tmp + 1; *p; ++p) {
        if (*p == '/') {
            *p = '\0';
            mkdir(tmp, 0755);
            *p = '/';
        }
    }
    mkdir(tmp, 0755);
}

MemCountAndPlan::MemCountAndPlan() {
    context = std::make_shared<MemContext>();
    sem_init(&sem_mem_align_created, 0, 0);
#ifdef MEM_STATS_ACTIVE
    mem_stats = new MemStats();
#endif
}

MemCountAndPlan::~MemCountAndPlan() {
    
    // Call clear
    clear();

#ifdef MEM_STATS_ACTIVE
    delete mem_stats;
#endif
}

static void generate_mem_segments_into(MemSegments dest[MEM_TYPES],
                                       const std::vector<InstanceMeta> &instances);

void MemCountAndPlan::clear() {
    // Wait for and clean up any background threads
    if (parallel_execute && parallel_execute->joinable()) {
        parallel_execute->join();
    }

    // Clean up count_workers raw pointers
    for (auto* worker : count_workers) {
        delete worker;
    }
    count_workers.clear();
    
    // Clean up plan_workers
    plan_threads.clear();
    
    // Clear segments (they have their own cleanup)
    for (int i = 0; i < MEM_TYPES; ++i) {
        segments[i].clear();
    }
    
    context->clear();
}
void MemCountAndPlan::prepare() {
#ifdef MEM_STATS_ACTIVE
    uint64_t init = get_usec();
#endif    
    // Clear existing workers to avoid memory leaks if prepare() called multiple times
    for (auto* worker : count_workers) {
        delete worker;
    }
    count_workers.clear();
    
    for (size_t i = 0; i < MAX_THREADS; ++i) {
        count_workers.push_back(new MemCounter(i, context));
#ifdef MEM_STATS_ACTIVE
        // Assign mem_stats to each worker if MEM_STATS_ACTIVE is defined
        count_workers[i]->mem_stats = mem_stats;
#endif // MEM_STATS_ACTIVE
    }
    mem_align_counter = std::make_unique<MemAlignCounter>(context);
    plan_workers.clear();
    plan_workers.reserve(MAX_MEM_PLANNERS);
    rom_data_planner = std::make_unique<ImmutableMemPlanner>(ROM_ROWS, ROM_ADDR, ROM_SIZE_MB, false);
    rom_data_planner->set_last_addr(ROM_ADDR - 8);
    input_data_planner = std::make_unique<ImmutableMemPlanner>(INPUT_ROWS, INPUT_ADDR, INPUT_SIZE_MB, false);
    quick_mem_planner = std::make_unique<MemPlanner>(0, RAM_ROWS, RAM_ADDR, RAM_SIZE_MB);
    for (int i = 0; i < MAX_MEM_PLANNERS; ++i) {
        plan_workers.emplace_back(i+1, RAM_ROWS, RAM_ADDR, RAM_SIZE_MB);
    }
#ifdef MEM_STATS_ACTIVE    
    t_prepare_us = get_usec() - init;
#endif
}

void MemCountAndPlan::add_chunk(MemCountersBusData *chunk_data, uint32_t chunk_size) {
    context->add_chunk(chunk_data, chunk_size);
    #ifdef SAVE_MEM_BUS_DATA_ASM
    save_chunk_data(context->size() - 1, chunk_data, chunk_size);
    #endif
}

void MemCountAndPlan::execute(void) {
    parallel_execute = std::make_unique<std::thread>(&MemCountAndPlan::detach_execute, this);
}

void MemCountAndPlan::detach_execute_mem_align_counter() {
    mem_align_counter->execute();
    #ifdef SAVE_MEM_ALIGN_COUNTERS
    const char *env_file = getenv("MEM_ALIGN_COUNTERS_FILE");
    std::string csv_path = env_file ? env_file : "tmp/mem_align_counters.csv";
    // Create parent directory if it doesn't exist
    size_t last_sep = csv_path.rfind('/');
    if (last_sep != std::string::npos) {
        std::string dir = csv_path.substr(0, last_sep);
        mkdir_recursive(dir.c_str());
    }
    mem_align_counter->save_csv(csv_path);
    #endif
}   
void MemCountAndPlan::count_phase() {

#ifdef MEM_STATS_ACTIVE
    // Get start time for stats
    struct timespec start_time;
    clock_gettime(CLOCK_REALTIME, &start_time);
#endif // MEM_STATS_ACTIVE

#ifdef MEM_STATS_ACTIVE
    uint64_t init = t_init_us = get_usec();
#endif
    std::vector<std::thread> threads;
    context->init();

    for (int i = 0; i < MAX_THREADS; ++i) {
        threads.emplace_back([this, i](){count_workers[i]->execute();});
    }
    // threads.emplace_back([this](){ mem_align_counter->execute();});
    mem_align_execute = std::make_unique<std::thread>(&MemCountAndPlan::detach_execute_mem_align_counter, this);
    if (sem_post(&sem_mem_align_created) != 0) {
        perror("sem_post");
    }

    for (auto& t : threads) {
        t.join();
    }

    uint64_t max_tot_wait_us = 0;
    uint64_t tot_wait_us = 0;
    uint32_t max_used_slots = 0;
    for (uint32_t index = 0; index < count_workers.size(); ++index) {
        if (count_workers[index]->tot_wait_us > max_tot_wait_us) {
            max_tot_wait_us = count_workers[index]->tot_wait_us;
        }
        tot_wait_us += count_workers[index]->tot_wait_us;        
        if (count_workers[index]->get_used_slots() > max_used_slots) {
            max_used_slots = count_workers[index]->get_used_slots();
        }
    }

    wait_mem_align_counters();

#ifdef MEM_STATS_ACTIVE
    t_count_us = (uint32_t) (get_usec() - init);

    // Add stats for count phase
    struct timespec end_time;
    clock_gettime(CLOCK_REALTIME, &end_time);
    assert(mem_stats != nullptr);
    mem_stats->add_stat(
        MEM_STATS_COUNT_PHASE,
        start_time.tv_sec,
        start_time.tv_nsec, 
        (end_time.tv_sec - start_time.tv_sec) * 1000000000 + (end_time.tv_nsec - start_time.tv_nsec));
#endif // MEM_STATS_ACTIVE
}

void MemCountAndPlan::plan_phase() {

#ifdef MEM_STATS_ACTIVE
    // Get start time for stats
    struct timespec start_time;
    clock_gettime(CLOCK_REALTIME, &start_time);

    uint64_t init = get_usec();
#endif // MEM_STATS_ACTIVE
    std::vector<std::thread> threads;

    plan_threads.emplace_back([this](){ quick_mem_planner->generate_locators(count_workers, context->locators);});
    plan_threads.emplace_back([this](){ rom_data_planner->execute(count_workers);});
    plan_threads.emplace_back([this](){ input_data_planner->execute(count_workers);});
    segments[RAM_ID].clear();
    for (int i = 0; i < MAX_MEM_PLANNERS; ++i) {
        threads.emplace_back([this, i](){ plan_workers[i].execute_from_locators(count_workers, context->locators, segments[RAM_ID]);});
    }
    for (auto& t : threads) {
        t.join();
    }
    for (auto& t : plan_threads) {
        t.join();
    }
#ifdef MEM_STATS_ACTIVE
    t_plan_us = (uint32_t) (get_usec() - init);
#endif
    segments[ROM_ID].clear();
    rom_data_planner->collect_segments(segments[ROM_ID]);

    segments[INPUT_ID].clear();
    input_data_planner->collect_segments(segments[INPUT_ID]);

#ifdef MEM_STATS_ACTIVE
    // Add stats for plan phase
    struct timespec end_time;
    clock_gettime(CLOCK_REALTIME, &end_time);
    assert(mem_stats != nullptr);
    mem_stats->add_stat(
        MEM_STATS_PLAN_PHASE,
        start_time.tv_sec,
        start_time.tv_nsec, 
        (end_time.tv_sec - start_time.tv_sec) * 1000000000 + (end_time.tv_nsec - start_time.tv_nsec));
#endif // MEM_STATS_ACTIVE
}

void MemCountAndPlan::stats() {
    uint32_t tot_used_slots = 0;
    for (size_t i = 0; i < MAX_THREADS; ++i) {
        uint32_t used_slots = count_workers[i]->get_used_slots();
        tot_used_slots += used_slots;
        printf("Thread %ld: used slots %d/%ld (%04.02f%%) T(ms):%d S(ms):%ld C0(us):%ld Q:%d\n",
            i, used_slots, ADDR_SLOTS,
            ((double)used_slots*100.0)/(double)(ADDR_SLOTS), count_workers[i]->get_elapsed_ms(),
            count_workers[i]->tot_wait_us/1000,
            count_workers[i]->get_first_chunk_us(),
            count_workers[i]->get_queue_full_times()/1000);
    }
    #ifdef CHUNK_STATS
    context->stats();
    for (size_t i = 0; i < MAX_THREADS; ++i) {
        count_workers[i]->stats();
    }
    #endif
    printf("\n> threads: %d\n", MAX_THREADS);
    printf("> address table: %ld MB\n", (ADDR_TABLE_SIZE * ADDR_TABLE_ELEMENT_SIZE * MAX_THREADS)>>20);
    printf("> memory slots: %ld MB (used: %ld MB)\n", (ADDR_SLOTS_SIZE * sizeof(uint32_t) * MAX_THREADS)>>20, (tot_used_slots * ADDR_SLOT_SIZE * sizeof(uint32_t))>> 20);
    printf("> page table: %ld MB\n\n", (ADDR_PAGE_SIZE * sizeof(uint32_t))>> 20);
    quick_mem_planner->stats();
    for (uint32_t i = 0; i < plan_workers.size(); ++i) {
        plan_workers[i].stats();
    }
    printf("prepare: %04.2f ms\n", t_prepare_us / 1000.0);
    printf("execution: %04.2f ms\n", (TIME_US_BY_CHUNK * context->size()) / 1000.0);
    printf("completed: %04.2f ms\n", context->get_completed_us() / 1000.0);
    printf("count_phase: %04.2f ms\n", t_count_us / 1000.0);
    printf("plan_phase: %04.2f ms\n", t_plan_us / 1000.0);
}

MemCountAndPlan *create_mem_count_and_plan(void) { 
    MemCountAndPlan *mcp = new MemCountAndPlan();
    mcp->prepare();
    return mcp;
}

void destroy_mem_count_and_plan(MemCountAndPlan *mcp) {
    if (mcp) {
        mcp->clear();
        delete mcp;
        mcp = nullptr;
    }
}

void execute_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->execute();
}

void save_chunk_data(uint32_t chunk_id, MemCountersBusData *chunk_data, uint32_t chunk_size)
{
    const char *env_dir = getenv("ASM_MOPS_DIR");
    const char *base_dir = env_dir ? env_dir : "tmp/asm_mops";

    mkdir_recursive(base_dir);

    char filename[512];
    snprintf(filename, sizeof(filename), "%s/mem_count_data_%d.bin", base_dir, chunk_id);
    int fd = open(filename, O_WRONLY | O_CREAT | O_TRUNC, S_IRUSR | S_IWUSR);
    if (fd < 0) {
        perror("Error opening file");
        return;
    }

    ssize_t bytes_written = write(fd, chunk_data, sizeof(MemCountersBusData) * chunk_size);
    if (bytes_written < 0) {
        perror("Error writing to file");
    } else if (static_cast<size_t>(bytes_written) != sizeof(MemCountersBusData) * chunk_size) {
        fprintf(stderr, "Partial write: expected %zu bytes, but wrote %zd bytes\n",
                sizeof(MemCountersBusData) * chunk_size, bytes_written);
    }

    close(fd);
}

void add_chunk_mem_count_and_plan(MemCountAndPlan *mcp, MemCountersBusData *chunk_data, uint32_t chunk_size)
{
     mcp->add_chunk(chunk_data, chunk_size);
}

void stats_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->stats();
}

void set_completed_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->set_completed();
}

void wait_mem_align_counters(MemCountAndPlan *mcp)
{
    mcp->wait_mem_align_counters();
}

void wait_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->wait();
}

// Pure generator: writes into the provided destination table.
static void generate_mem_segments_into(MemSegments dest[MEM_TYPES], const std::vector<InstanceMeta> &instances) {
    uint32_t last_segments[MEM_TYPES];
    for (int i = 0; i < MEM_TYPES; ++i) {
        last_segments[i] = 0;
        dest[i].clear();
    }

    for (const auto &instance : instances) {
        if (instance.inst_id >= last_segments[instance.kind]) {
            last_segments[instance.kind] = instance.inst_id;
        }
    }
    const int64_t n_inst = static_cast<int64_t>(instances.size());
    #pragma omp parallel for schedule(dynamic) num_threads(4)
    for (int64_t i = 0; i < n_inst; ++i) {
        const InstanceMeta &instance = instances[i];
        MemSegment *segment = new MemSegment();
        uint32_t first_chunk = instance.first_addr_chunk;
        uint32_t last_chunk = instance.last_addr_chunk;

        for (uint32_t chunk_id = 0; chunk_id < instance.n_chunks; ++chunk_id) {
            uint32_t count = instance.count_per_chunk[chunk_id];
            if (count == 0) continue;

            uint32_t from_addr = instance.first_addr;
            uint32_t skip = 0;
            uint32_t to_addr = instance.last_addr;

            if (chunk_id < first_chunk) {
                from_addr += 8;
            } else if (chunk_id == first_chunk && instance.inst_id > 0) {
                skip = instance.first_addr_skip + 1;
            }

            uint32_t to_count = UINT32_MAX;
            if (chunk_id == last_chunk) {
                to_count = instance.last_addr_include;
            } else if (chunk_id > last_chunk) {
                to_addr -= 8;
            }
            segment->push(chunk_id, from_addr, skip, to_addr, to_count, count);
            if (chunk_id == first_chunk && instance.inst_id > 0) {
                segment->swap_last_and_first();
            }
        }
        segment->is_last_segment = instance.inst_id == last_segments[instance.kind];
        segment->offsets_base_addr = instance.first_addr;
        segment->offsets = instance.offsets;
        dest[instance.kind].set(instance.inst_id, segment);
    }
}

// Inject GPU-produced metas straight into `mcp->segments[]`. The GPU planner
// owns the per-meta `count_per_chunk` / `page_starts` / `page_single_value` /
// `pages_dense` arrays and must remain alive until the segments are consumed.
// The shallow vector copy here just gives the segment generator the
// `vector<InstanceMeta>` shape it expects; the pointers inside are untouched.
bool inject_gpu_metas_from_pointers(MemCountAndPlan *mcp, const void *gpu_metas_ptr, uint32_t n) {
    if (!mcp || (!gpu_metas_ptr && n != 0)) return false;
    const InstanceMeta *gpu_metas = static_cast<const InstanceMeta *>(gpu_metas_ptr);

    // Validate every meta before trusting it
    for (uint32_t i = 0; i < n; ++i) {
        const InstanceMeta &m = gpu_metas[i];
        if (m.n_chunks > MAX_CHUNKS) {
            fprintf(stderr,
                    "inject_gpu_metas_from_pointers: meta %u has n_chunks=%u > MAX_CHUNKS=%u; reject\n",
                    i, m.n_chunks, (uint32_t)MAX_CHUNKS);
            return false;
        }
        if (m.n_chunks != 0 && m.count_per_chunk == nullptr) {
            fprintf(stderr,
                    "inject_gpu_metas_from_pointers: meta %u has n_chunks=%u but count_per_chunk is null; reject\n",
                    i, m.n_chunks);
            return false;
        }
        if (m.kind >= MEM_TYPES) {
            fprintf(stderr,
                    "inject_gpu_metas_from_pointers: meta %u has kind=%u >= MEM_TYPES=%u; reject\n",
                    i, m.kind, (uint32_t)MEM_TYPES);
            return false;
        }
    }
    std::vector<InstanceMeta> metas(gpu_metas, gpu_metas + n);
    generate_mem_segments_into(mcp->segments, metas);
    return true;
}

uint32_t get_mem_segment_count(MemCountAndPlan *mcp, uint32_t mem_id)
{
    if (mem_id >= MEM_TYPES) return 0;
    return mcp->segments[mem_id].size();
}

const MemCheckPoint *get_mem_segment_check_points(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id, uint32_t &count)
{
    if (mem_id >= MEM_TYPES) { count = 0; return nullptr; }
    auto segment = mcp->segments[mem_id].get(segment_id);
    if (!segment) { count = 0; return nullptr; }
    count = segment->size();
    return segment->get_chunks();
}

PagedOffsets get_mem_segment_offset_pages(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id,
                                          uint32_t &offsets_base_addr_out)
{
    if (mem_id >= MEM_TYPES) { offsets_base_addr_out = 0; return PagedOffsets{nullptr, nullptr, nullptr, 0, 0, 0}; }
    auto segment = mcp->segments[mem_id].get(segment_id);
    if (segment) {
        offsets_base_addr_out = segment->offsets_base_addr;
        return segment->offsets;
    }
    offsets_base_addr_out = 0;
    return PagedOffsets{nullptr, nullptr, nullptr, 0, 0, 0};
}

const MemAlignChunkCounters *get_mem_align_counters(MemCountAndPlan *mcp, uint32_t &count)
{
    count = mcp->mem_align_counter->size();
    if (count == 0) {
        return nullptr;
    }
    return mcp->mem_align_counter->get_counters();
}

const MemAlignChunkCounters *get_mem_align_total_counters(MemCountAndPlan *mcp)
{
    return mcp->mem_align_counter->get_total_counters();
}

void MemCountAndPlan::wait_mem_align_counters() {
    while (sem_wait(&sem_mem_align_created) < 0) {
        if (errno != EINTR) {
            throw std::runtime_error("MemContext::wait_mem_align_counters: sem_wait error");
        }
    }

    try {
        if (mem_align_execute && mem_align_execute->joinable()) {
            mem_align_execute->join();
        }
    } catch (const std::exception &e) {
        printf("Exception mem_align_execute in wait: %s\n", e.what());
    }
    sem_post(&sem_mem_align_created);   
}

void MemCountAndPlan::wait() {
    // GPU-mode callers skip `execute()`, so `parallel_execute` is never spawned. 
    if (!parallel_execute || !parallel_execute->joinable()) return;
    try {
        parallel_execute->join();
    } catch (const std::exception &e) {
        printf("Exception parallel_execute wait: %s\n", e.what());
    }
}

void MemCountAndPlan::detach_execute() {
    count_phase();
    plan_phase();
    // stats();
    // printf("MemCountAndPlan count(ms):%ld plan(ms):%ld tot(ms):%ld\n", 
    //        t_count_us / 1000, t_plan_us / 1000, (t_count_us + t_plan_us) / 1000);
}


uint64_t get_mem_stats_len(MemCountAndPlan *mcp)
{
#ifdef MEM_STATS_ACTIVE
    return mcp->mem_stats->stats.size();
#else
    (void)mcp; // To avoid unused parameter warning
    return 0; // If MEM_STATS_ACTIVE is not defined, return 0
#endif // MEM_STATS_ACTIVE
}

uint64_t get_mem_stats_ptr(MemCountAndPlan * mcp)
{
#ifdef MEM_STATS_ACTIVE
    return (uint64_t)mcp->mem_stats->stats.data();
#else
    (void)mcp; // To avoid unused parameter warning
    return 0; // If MEM_STATS_ACTIVE is not defined, return 0
#endif // MEM_STATS_ACTIVE
}