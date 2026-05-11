#include <memory>
#include <algorithm>
#include "api.hpp"
#include "tools.hpp"
#include "mem_count_and_plan.hpp"
#include "mem_stats.hpp"
#include "instance_meta_loader.hpp"

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

void generate_mem_segments_from_gpu_plan(MemCountAndPlan *mcp, const std::vector<InstanceMeta> instances);
static bool read_bin_offsets_file(const char *filename, std::vector<uint32_t> &out_offsets);
void compare_bin_offsets_to_segment(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id);
void load_mem_metas_and_generate_segments(MemCountAndPlan *mcp);

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

uint32_t *read_counters_from_bin_file(const char *filename, uint32_t &count)
{
    int fd = open(filename, O_RDONLY);
    if (fd < 0) {
        perror("Error opening file");
        count = 0;
        return nullptr;
    }

    off_t file_size = lseek(fd, 0, SEEK_END);
    lseek(fd, 0, SEEK_SET);

    if (file_size <= 0) {
        close(fd);
        count = 0;
        return nullptr;
    }

    count = static_cast<uint32_t>(file_size / sizeof(uint32_t));
    uint32_t *data = static_cast<uint32_t *>(malloc(file_size));
    if (!data) {
        close(fd);
        count = 0;
        return nullptr;
    }

    ssize_t bytes_read = read(fd, data, file_size);
    if (bytes_read < 0 || static_cast<size_t>(bytes_read) != static_cast<size_t>(file_size)) {
        perror("Error reading file");
        free(data);
        close(fd);
        count = 0;
        return nullptr;
    }

    close(fd);
    return data;
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

void load_memalign_counters_from_file_and_compare(MemCountAndPlan *mcp)
{
    uint32_t counters_count;
    uint32_t *file_counters = read_counters_from_bin_file("tmp/mem_align_counters.bin", counters_count);
    const MemAlignChunkCounters *counters = mcp->mem_align_counter->get_counters();
    for (int i = 0; i < mcp->mem_align_counter->size(); ++i) {
        uint32_t chunk_id = counters[i].chunk_id;
        uint32_t index = chunk_id * 5;
        bool equal = file_counters[index + 0] == counters[i].full_5 &&
                     file_counters[index + 1] == counters[i].full_3 &&
                     file_counters[index + 2] == counters[i].full_2 &&
                     file_counters[index + 3] == counters[i].read_byte &&
                     file_counters[index + 4] == counters[i].write_byte;
        if (!equal) {
            printf("DIFF chunk %d: file [%d, %d, %d, %d, %d] counter [%d, %d, %d, %d, %d]\n", i,
                file_counters[index + 0], file_counters[index + 1], file_counters[index + 2], file_counters[index + 3], file_counters[index + 4],
                counters[i].chunk_id, counters[i].full_5, counters[i].full_3, counters[i].full_2, counters[i].read_byte, counters[i].write_byte);
        }
    }
}
void wait_mem_align_counters(MemCountAndPlan *mcp)
{
    mcp->wait_mem_align_counters();
    load_memalign_counters_from_file_and_compare(mcp);
}

void wait_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->wait();
    load_mem_metas_and_generate_segments(mcp);
}
void load_mem_metas_and_generate_segments(MemCountAndPlan *mcp)
{
    std::call_once(mcp->wait_once, [mcp]() {
        printf("MemCountAndPlan execution completed.\nLoading metas...\n");
        struct LoadedMetas metas = load_instance_metas("tmp/metas.bin");
        std::sort(metas.metas.begin(), metas.metas.end(),
                  [](const InstanceMeta &a, const InstanceMeta &b) { return a.type < b.type || (a.type == b.type && a.inst_id < b.inst_id); });
        for (const auto &meta : metas.metas) {
            uint32_t count_zeros = 0;
            for (size_t i = 0; i < meta.addr_offsets.size(); ++i) {
                if (meta.addr_offsets[i] == 0) {
                    count_zeros++;
                }
            }
            printf("Instance %d: type=%d first_addr=0x%08X last_addr=0x%08X first_chunk=%d first_skip=%d last_chunk=%d last_include=%d count_per_chunk_len=%zu addr_offsets_len=%zu zeros=%d/%d\n",
                meta.inst_id, meta.type, meta.first_addr, meta.last_addr, meta.first_addr_chunk, meta.first_addr_skip,
                meta.last_addr_chunk, meta.last_addr_include, meta.count_per_chunk.size(), meta.addr_offsets.size(), count_zeros, meta.addr_offsets.size());
        }
        printf("Metas loaded (%zu).\n", metas.metas.size());
        generate_mem_segments_from_gpu_plan(mcp, metas.metas);
    });
}

void generate_mem_segments_from_gpu_plan(MemCountAndPlan *mcp, const std::vector<InstanceMeta> instances) {
    uint32_t last_segments[MEM_TYPES];
    for (int i = 0; i < MEM_TYPES; ++i) {
        last_segments[i] = 0;
        mcp->segments[i].clear();
    }

    for (const auto &instance : instances) {    
        if (instance.inst_id >= last_segments[instance.type]) {
            last_segments[instance.type] = instance.inst_id;
        }
    }
    for (const auto &instance : instances) {   
        MemSegment *segment = new MemSegment();
        uint32_t first_chunk = instance.first_addr_chunk;
        uint32_t last_chunk = instance.last_addr_chunk;


        for (uint32_t chunk_id = 0; chunk_id < instance.count_per_chunk.size(); ++chunk_id) {            
            uint32_t count = instance.count_per_chunk[chunk_id];
            if (count == 0) continue;

            uint32_t from_addr = instance.first_addr;
            uint32_t skip = 0;
            uint32_t to_addr = instance.last_addr;
            
            if (chunk_id < first_chunk) {
                from_addr += 8;
            } else if (chunk_id == first_chunk && instance.inst_id > 0) {
                // The first instance cannot have skip; subsequent instances will have
                // the halo entry and it must be discarded.
                skip = instance.first_addr_skip + 1;
            }

            // An alternative to include all of them would be: to_addr + 8 and count = 0
            
            uint32_t to_count = UINT32_MAX;            
            if (chunk_id == last_chunk) {                
                to_count = instance.last_addr_include;
            } else if (chunk_id > last_chunk) {
                to_addr -= 8;
            }
            // printf("MEM%d @%d#%d [0x%08X s:%d] [0x%08X C:%d] C:%d\n",
            //     instance.type, instance.inst_id, chunk_id, from_addr, skip, to_addr, to_count, count);
            segment->push(chunk_id, from_addr, skip, to_addr, to_count, count);
            if (chunk_id == first_chunk && instance.inst_id > 0) {
                segment->swap_last_and_first();
            }
        }
        segment->is_last_segment = instance.inst_id == last_segments[instance.type];
        segment->offsets_base_addr = instance.first_addr;
        if (instance.type == RAM_ID) {
            char filename[512];
            snprintf(filename, sizeof(filename), "tmp/mem_trace_%04u_bin_offsets.bin", instance.inst_id);
            // std::vector<uint32_t> loaded;
            // if (read_bin_offsets_file(filename, loaded)) {
            //     segment->offsets = std::move(loaded);
            // } else {
            //     fprintf(stderr, "generate_mem_segments_from_gpu_plan: falling back to GPU offsets for RAM segment %u\n", instance.inst_id);
            //     segment->offsets.assign(instance.addr_offsets.begin(), instance.addr_offsets.end());
            // }
            segment->offsets.assign(instance.addr_offsets.begin(), instance.addr_offsets.end());
        } else {
            segment->offsets.assign(instance.addr_offsets.begin(), instance.addr_offsets.end());
        }
        mcp->segments[instance.type].set(instance.inst_id, segment);
    }
    // printf("Mem segments generated from GPU plan: ROM=%zu, INPUT=%zu, RAM=%zu\n", 
    //     mcp->segments[ROM_ID].size(), mcp->segments[INPUT_ID].size(), mcp->segments[RAM_ID].size());
    // for (uint32_t segment_id = 0; segment_id < mcp->segments[RAM_ID].size(); ++segment_id) {
    //     compare_bin_offsets_to_segment(mcp, RAM_ID, segment_id);
    // }
    // for (int i = 0; i < MEM_TYPES; ++i) {
    //     segments[i].compare(mcp->segments[i]);
    // }
}

uint32_t get_mem_segment_count(MemCountAndPlan *mcp, uint32_t mem_id)
{
    return mcp->segments[mem_id].size();
}

const MemCheckPoint *get_mem_segment_check_points(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id, uint32_t &count)
{
    auto segment = mcp->segments[mem_id].get(segment_id);
    count = segment ? segment->size() : 0;
    return segment->get_chunks();
}

const uint32_t *get_mem_segment_offsets(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id, uint32_t &offsets_base_addr, uint32_t &count)
{
    auto segment = mcp->segments[mem_id].get(segment_id);
    if (segment) {
        offsets_base_addr = segment->offsets_base_addr;
        count = segment->offsets.size();
        return segment->offsets.data();
    } else {
        offsets_base_addr = 0;
        count = 0;
        return nullptr;
    }
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

// ---------------------------------------------------------------------------
// Binary-offsets helpers
//
// Both functions expect files written by MemSM::save_bin_offsets_to_file
// (Rust), whose binary layout is:
//   u32 offsets_base_addr  – byte address of the first qword entry
//   u32 num_entries        – number of qword slots
//   u32[num_entries]       – 1-based row indices (0 = address not present)
// ---------------------------------------------------------------------------

static bool read_bin_offsets_file(const char *filename,
                                   std::vector<uint32_t> &out_offsets)
{
    struct stat st;
    if (stat(filename, &st) != 0) {
        fprintf(stderr, "read_bin_offsets_file: cannot stat '%s': %s\n",
                filename, strerror(errno));
        return false;
    }
    if (st.st_size % sizeof(uint32_t) != 0) {
        fprintf(stderr, "read_bin_offsets_file: file size %lld is not a multiple of 4 in '%s'\n",
                (long long)st.st_size, filename);
        return false;
    }
    uint32_t count = (uint32_t)(st.st_size / sizeof(uint32_t));
    FILE *f = fopen(filename, "rb");
    if (!f) {
        fprintf(stderr, "read_bin_offsets_file: cannot open '%s': %s\n",
                filename, strerror(errno));
        return false;
    }
    out_offsets.resize(count);
    if (fread(out_offsets.data(), sizeof(uint32_t), count, f) != count) {
        fprintf(stderr, "read_bin_offsets_file: failed to read %u entries from '%s'\n",
                count, filename);
        fclose(f);
        return false;
    }
    fclose(f);
    return true;
}

// Load binary offsets produced by the legacy CPU path and overwrite the
// offsets stored in the corresponding MemSegment (segment->offsets).
// The base address (segment->offsets_base_addr) is NOT changed; it must
// already be set to the correct value (e.g. from instance.first_addr).
void load_bin_offsets_to_segment(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id)
{
    char filename[512];
    snprintf(filename, sizeof(filename), "tmp/mem_trace_%u_bin_offsets.bin", segment_id);

    std::vector<uint32_t> offsets;
    if (!read_bin_offsets_file(filename, offsets)) {
        return;
    }

    auto it = mcp->segments[mem_id].segments.find(segment_id);
    if (it == mcp->segments[mem_id].segments.end()) {
        fprintf(stderr, "load_bin_offsets_to_segment: segment %u not found in mem_id %u\n",
                segment_id, mem_id);
        return;
    }
    MemSegment *segment = it->second;
    segment->offsets = std::move(offsets);
    printf("load_bin_offsets_to_segment: segment=%u mem_id=%u loaded %zu offsets (base_addr=0x%08X)\n",
           segment_id, mem_id, segment->offsets.size(), segment->offsets_base_addr);
}

// Compare the binary offsets file (legacy CPU path) against the offsets
// currently stored in a MemSegment and print every difference as:
//   DIFF inst=<id> addr=0x<byte_addr> offset_calculated=<seg> offset_from_bin=<file>
//
// Both the file and the segment share the same base address
// (segment->offsets_base_addr); the file carries no header.
void compare_bin_offsets_to_segment(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id)
{
    char filename[512];
    snprintf(filename, sizeof(filename), "tmp/mem_trace_%u_bin_offsets.bin", segment_id);

    std::vector<uint32_t> file_offsets;
    if (!read_bin_offsets_file(filename, file_offsets)) {
        return;
    }

    const MemSegment *segment = mcp->segments[mem_id].get(segment_id);
    if (!segment) {
        fprintf(stderr, "compare_bin_offsets_to_segment: segment %u not found in mem_id %u\n",
                segment_id, mem_id);
        return;
    }

    const uint32_t base_addr = segment->offsets_base_addr;
    const std::vector<uint32_t> &seg_offsets = segment->offsets;

    // Both arrays share the same base; compare over the shorter range.
    const uint32_t cmp_count = (uint32_t)std::min(file_offsets.size(), seg_offsets.size());

    uint32_t diffs = 0;
    for (uint32_t i = 0; i < cmp_count; ++i) {
        const uint32_t file_off = file_offsets[i];
        const uint32_t seg_off  = seg_offsets[i];
        if (file_off != seg_off) {
            const uint32_t byte_addr = base_addr + i * 8;
            printf("DIFF inst=%u index=%u addr=0x%08X offset_calculated=%u offset_from_bin=%u\n",
                   segment_id, i, byte_addr, seg_off, file_off);
            ++diffs;
        }
    }
    if (file_offsets.size() != seg_offsets.size()) {
        printf("compare_bin_offsets_to_segment: segment=%u mem_id=%u size mismatch: file=%zu seg=%zu\n",
               segment_id, mem_id, file_offsets.size(), seg_offsets.size());
    }
    if (diffs == 0 && file_offsets.size() == seg_offsets.size()) {
        printf("compare_bin_offsets_to_segment: segment=%u mem_id=%u - no differences\n",
               segment_id, mem_id);
    } else {
        printf("compare_bin_offsets_to_segment: segment=%u mem_id=%u - %u differences found\n",
               segment_id, mem_id, diffs);
    }
}