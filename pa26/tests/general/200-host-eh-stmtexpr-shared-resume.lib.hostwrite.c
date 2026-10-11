#include <unistd.h>
long cppgm_test_write_stdout(const char* buf, unsigned long count) { return write(1, buf, count); }
