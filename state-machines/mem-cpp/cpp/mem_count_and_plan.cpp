
#include "mem_count_and_plan.hpp"

void MemCountAndPlan::wait() {
    try {
        printf("WAIT EXECUTE\n");
        printf("WAIT JOINABLE %d\n", parallel_execute->joinable());
        parallel_execute->join();
        printf("WAIT EXECUTE 1\n");
        // delete parallel_execute;
        printf("WAIT EXECUTE 2\n");
        // parallel_execute = nullptr;
        printf("WAIT END 0\n");
        printf("WAIT END 1\n");
        printf("WAIT END 2\n");
        printf("WAIT END 3\n");
        printf("WAIT END 4\n");
        printf("WAIT END 5\n");
        printf("WAIT END 6\n");
        sleep(1);
    } catch (const std::exception &e) {
        printf("Exception in wait: %s\n", e.what());
    }
}


void MemCountAndPlan::detach_execute() {
    printf("MemCountAndPlan::count_phase\n");
    count_phase();
    printf("MemCountAndPlan::plan_phase\n");
    plan_phase();
    printf("MemCountAndPlan::END\n");
}
