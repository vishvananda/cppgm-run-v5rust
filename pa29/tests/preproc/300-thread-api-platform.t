#if defined(__APPLE__) || defined(__linux__) || defined(__FreeBSD__) || defined(__NetBSD__) || defined(__OpenBSD__) || defined(__NuttX__) || defined(__GNU__) || defined(__MVS__) || defined(_AIX) || defined(__EMSCRIPTEN__)
thread_api_platform
#else
missing_thread_api_platform
#endif
