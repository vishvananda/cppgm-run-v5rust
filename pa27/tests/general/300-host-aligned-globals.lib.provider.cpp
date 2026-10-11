#include <cstdint>
#include <cstdio>
struct alignas(16) Block { int value; };
extern int table[4];
extern int counter;
extern Block block;
extern int wide;
extern int zeros[3];
extern long cleared[2];
static int offset(const void *p, unsigned align) { return (int)((std::uintptr_t)p % align); }
int host_check()
{
  std::printf("%d %d %d %d %d %d\n", offset(table, 32), offset(&counter, 64), offset(&block, 16),
    offset(&wide, 32), offset(zeros, 4), offset(cleared, 16));
  return table[1] + counter + block.value + wide + zeros[0] + (int)cleared[1] + 3;
}
