
#include "mem_count_and_plan.hpp"

void MemCountAndPlan::wait() {
    try {
        parallel_execute->join();
        // delete parallel_execute;
        // parallel_execute = nullptr;
    } catch (const std::exception &e) {
        printf("Exception in wait: %s\n", e.what());
    }
}


void MemCountAndPlan::detach_execute() {
    count_phase();
    plan_phase();
}
