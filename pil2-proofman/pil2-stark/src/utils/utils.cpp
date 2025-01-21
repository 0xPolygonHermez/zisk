#include <fstream>
#include <iostream>
#include <iomanip>
#include <filesystem>
#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/types.h>
#include <unistd.h>
#include <execinfo.h>
#include <ifaddrs.h>
#include <net/if.h>
#include <arpa/inet.h>
#include "utils.hpp"

using namespace std;
using namespace std::filesystem;

void printCallStack(void)
{
    void *callStack[100];
    size_t callStackSize = backtrace(callStack, 100);
    char **callStackSymbols = backtrace_symbols(callStack, callStackSize);
    zklog.info("CALL STACK");
    for (uint64_t i = 0; i < callStackSize; i++)
    {
        zklog.info(to_string(i) + ": call=" + callStackSymbols[i]);
    }
    free(callStackSymbols);
}

void getMemoryInfo(MemoryInfo &info)
{
    vector<string> labels{"MemTotal:", "MemFree:", "MemAvailable:", "Buffers:", "Cached:", "SwapCached:", "SwapTotal:", "SwapFree:"};

    ifstream meminfo = ifstream{"/proc/meminfo"};
    if (!meminfo.good())
    {
        zklog.error("Failed to get memory info");
    }

    string line, label;
    uint64_t value;
    while (getline(meminfo, line))
    {
        stringstream ss{line};
        ss >> label >> value;
        if (find(labels.begin(), labels.end(), label) != labels.end())
        {
            if (label == "MemTotal:") info.total = value;
            else if (label == "MemFree:") info.free = value;
            else if (label == "MemAvailable:") info.available = value;
            else if (label == "Buffers:") info.buffers = value;
            else if (label == "Cached:") info.cached = value;
            else if (label == "SwapCached:") info.swapCached = value;
            else if (label == "SwapTotal:") info.swapTotal = value;
            else if (label == "SwapFree:") info.swapFree = value;
        }
    }
    meminfo.close();
}

void parseProcSelfStat (double &vm, double &rss)
{
    string aux;
    ifstream ifs("/proc/self/stat", ios_base::in);
    ifs >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> aux >> vm >> rss;
}

void printMemoryInfo(bool compact, const char * pMessage)
{
    string s;

    string endLine = (compact ? ", " : "\n");
    string tab = (compact ? "" : "    ");

    s = "MEMORY INFO " + (pMessage==NULL?"":string(pMessage)) + endLine;

    constexpr double factorMB = 1024;

    MemoryInfo info;
    getMemoryInfo(info);

    double vm, rss;
    parseProcSelfStat(vm, rss);
    vm /= 1024*1024;
    rss /= 1024*1024;

    s += tab + "MemTotal: "+ to_string(info.total / factorMB) + " MB" + endLine;
    s += tab + "MemFree: " + to_string(info.free / factorMB) + " MB" + endLine;
    s += tab + "MemAvailable: " + to_string(info.available / factorMB) + " MB" + endLine;
    s += tab + "Buffers: " + to_string(info.buffers / factorMB) + " MB" + endLine;
    s += tab + "Cached: " + to_string(info.cached / factorMB) + " MB" + endLine;
    s += tab + "SwapCached: " + to_string(info.swapCached / factorMB) + " MB" + endLine;
    s += tab + "SwapTotal: " + to_string(info.swapTotal / factorMB) + " MB" + endLine;
    s += tab + "SwapFree: " + to_string(info.swapFree / factorMB) + " MB" + endLine;
    s += tab + "VM: " + to_string(vm) + " MB" + endLine;
    s += tab + "RSS: " + to_string(rss) + " MB";

    zklog.info(s);
}

void printProcessInfo(bool compact)
{
    string endLine = (compact ? ", " : "\n");
    string tab = (compact ? "" : "    ");

    string s = "PROCESS INFO" + endLine;

    ifstream stat("/proc/self/stat", ios_base::in);
    if (!stat.good())
    {
        zklog.error("printProcessInfo() failed to get process stat info");
        return;
    }

    string comm, state, ppid, pgrp, session, tty_nr;
    string tpgid, flags, minflt, cminflt, majflt, cmajflt;
    string cutime, cstime, priority, nice;
    string itrealvalue, starttime;

    int pid;
    unsigned long utime, stime, vsize;
    long rss, numthreads;

    stat >> pid >> comm >> state >> ppid >> pgrp >> session >> tty_nr >> tpgid >> flags >> minflt >> cminflt >> majflt >> cmajflt >> utime >> stime >> cutime >> cstime >> priority >> nice >> numthreads >> itrealvalue >> starttime >> vsize >> rss;

    stat.close();

    s += tab + "Pid: " + to_string(pid) + endLine;
    s += tab + "User time: " + to_string((double)utime / sysconf(_SC_CLK_TCK)) + " s" + endLine;
    s += tab + "Kernel time: " + to_string((double)stime / sysconf(_SC_CLK_TCK)) + " s" + endLine;
    s += tab + "Total time: " + to_string((double)utime / sysconf(_SC_CLK_TCK) + (double)stime / sysconf(_SC_CLK_TCK)) + " s" + endLine;
    s += tab + "Num threads: " + to_string(numthreads) + endLine;
    s += tab + "Virtual mem: " + to_string(vsize / 1024 / 1024) + " MB";

    zklog.info(s);
}

string getTimestamp(void)
{
    struct timeval tv;
    gettimeofday(&tv, NULL);
    char tmbuf[64], buf[256];
    strftime(tmbuf, sizeof(tmbuf), "%Y%m%d_%H%M%S", gmtime(&tv.tv_sec));
    snprintf(buf, sizeof(buf), "%s_%06ld", tmbuf, tv.tv_usec);
    return buf;
}

string getTimestampWithPeriod(void)
{
    struct timeval tv;
    gettimeofday(&tv, NULL);
    char buf[256];
    snprintf(buf, sizeof(buf), "%ld.%06ld", tv.tv_sec, tv.tv_usec);
    return buf;
}

void json2file(const json &j, const string &fileName)
{
    ofstream outputStream(fileName);
    if (!outputStream.good())
    {
        zklog.error("json2file() failed creating output JSON file " + fileName);
        exitProcess();
    }
    outputStream << setw(4) << j << endl;
    outputStream.close();
}

void file2json(const string &fileName, json &j)
{
    // zklog.info("file2json() (ordered) loading JSON file " + fileName);
    std::ifstream inputStream(fileName);
    if (!inputStream.good())
    {
        zklog.error("file2json() failed loading input JSON file " + fileName);
        exitProcess();
    }
    try
    {
        inputStream >> j;
    }
    catch (exception &e)
    {
        zklog.error("file2json() failed parsing input JSON file " + fileName + " exception=" + e.what());
        exitProcess();
    }
    inputStream.close();
}

bool fileExists (const string &fileName)
{
    struct stat fileStat;
    int iResult = stat( fileName.c_str(), &fileStat);
    return (iResult == 0);
}

uint64_t fileSize (const string &fileName)
{
    struct stat fileStat;
    int iResult = stat( fileName.c_str(), &fileStat);
    if (iResult != 0)
    {
        zklog.error("fileSize() could not find file " + fileName);
        exitProcess();
    }
    return fileStat.st_size;
}


void loadFileParallel(void* buffer, const string &fileName, uint64_t size) {

    // Check file size
    struct stat sb;
    if (lstat(fileName.c_str(), &sb) == -1) {
        zklog.error("loadFileParallel() failed calling lstat() of file " + fileName);
        exitProcess();
    }
    if ((uint64_t)sb.st_size != size) {
        zklog.error("loadFileParallel() found size of file " + fileName + " to be " + to_string(sb.st_size) + " B instead of " + to_string(size) + " B");
        exitProcess();
    }

    // Determine the number of chunks and the size of each chunk
    size_t numChunks = 8; //omp_get_max_threads()/2;
    if(numChunks == 0 ) numChunks = 1;
    size_t chunkSize = size / numChunks;
    size_t remainder = size - numChunks*chunkSize;
    
    #pragma omp parallel for num_threads(numChunks)
    for(size_t i=0; i<numChunks; i++){
        // Open the file
        FILE* file = fopen(fileName.c_str(), "rb");
        if(file == NULL){
            zklog.error("loadFileParallel() failed to open the file");
            exitProcess();
        }
        size_t chunkSize_ = i == numChunks -1 ? chunkSize + remainder : chunkSize;
        size_t offset = i * chunkSize;
        fseek(file, offset, SEEK_SET);
        size_t readed = fread((uint8_t*)buffer + offset, 1, chunkSize_, file);
        if(readed != chunkSize_){
            zklog.error("loadFileParallel() failed to read the file");
        }
        fclose(file);
    }
}

void* loadFileParallel(const string &fileName, uint64_t size) {

    // Check file size
    struct stat sb;
    if (lstat(fileName.c_str(), &sb) == -1) {
        zklog.error("loadFileParallel() failed calling lstat() of file " + fileName);
        exitProcess();
    }
    if ((uint64_t)sb.st_size != size) {
        zklog.error("loadFileParallel() found size of file " + fileName + " to be " + to_string(sb.st_size) + " B instead of " + to_string(size) + " B");
        exitProcess();
    }

    // Allocate memory
    void* buffer = malloc(size);
    if (buffer == NULL) {
        zklog.error("loadFileParallel() failed calling malloc() of size: " + to_string(size));
        exitProcess();
    }

    // Determine the number of chunks and the size of each chunk
    size_t numChunks = 8; //omp_get_max_threads()/2;
    if(numChunks == 0 ) numChunks = 1;
    size_t chunkSize = size / numChunks;
    size_t remainder = size - numChunks*chunkSize;
    
    #pragma omp parallel for num_threads(numChunks)
    for(size_t i=0; i<numChunks; i++){
        // Open the file
        FILE* file = fopen(fileName.c_str(), "rb");
        if(file == NULL){
            zklog.error("loadFileParallel() failed to open the file");
            exitProcess();
        }
        size_t chunkSize_ = i == numChunks -1 ? chunkSize + remainder : chunkSize;
        size_t offset = i * chunkSize;
        fseek(file, offset, SEEK_SET);
        size_t readed = fread((uint8_t*)buffer + offset, 1, chunkSize_, file);
        if(readed != chunkSize_){
            zklog.error("loadFileParallel() failed to read the file");
        }
        fclose(file);
    }

    return buffer;
}

void writeFileParallel(const string &fileName, const void* buffer, uint64_t size, uint64_t offset) {
    // Determine the number of chunks and the size of each chunk
    size_t numChunks = 8;  // or omp_get_max_threads() / 2;
    if (numChunks == 0) numChunks = 1;
    size_t chunkSize = size / numChunks;
    size_t remainder = size - numChunks * chunkSize;

    #pragma omp parallel for num_threads(numChunks)
    for (size_t i = 0; i < numChunks; i++) {
        // Open the file in read/write binary mode to allow concurrent writing
        FILE* file = fopen(fileName.c_str(), "r+b");
        if (file == NULL) {
            std::cerr << "writeFileParallel() failed to open the file" << std::endl;
            exit(EXIT_FAILURE);
        }

        // Calculate the chunk size for each thread, with the last chunk handling any remainder
        size_t chunkSize_ = (i == numChunks - 1) ? chunkSize + remainder : chunkSize;
        size_t chunkOffset = offset + i * chunkSize;

        // Seek to the position where this chunk should be written
        fseek(file, chunkOffset, SEEK_SET);

        // Write the chunk from the buffer to the file
        size_t written = fwrite((const uint8_t*)buffer + i * chunkSize, 1, chunkSize_, file);
        if (written != chunkSize_) {
            std::cerr << "writeFileParallel() failed to write the file" << std::endl;
        }

        // Close the file after writing the chunk
        fclose(file);
    }
}

void unmapFile(void *pAddress, uint64_t size)
{
    int err = munmap(pAddress, size);
    if (err != 0)
    {
        zklog.error("unmapFile() failed calling munmap() of address=" + to_string(uint64_t(pAddress)) + " size=" + to_string(size));
        exitProcess();
    }
}
