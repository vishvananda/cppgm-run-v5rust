#if !defined(__x86_64__)
#error "pa24 no-host runtime helpers currently require x86_64"
#endif

#if defined(__APPLE__)
#define CPPGM_TEST_SYSCALL_READ 0x2000003UL
#define CPPGM_TEST_SYSCALL_WRITE 0x2000004UL
#define CPPGM_TEST_SYSCALL_MMAP 0x20000C5UL
#define CPPGM_TEST_MMAP_FLAGS 0x1002UL
#elif defined(__linux__)
#define CPPGM_TEST_SYSCALL_READ 0UL
#define CPPGM_TEST_SYSCALL_WRITE 1UL
#define CPPGM_TEST_SYSCALL_MMAP 9UL
#define CPPGM_TEST_MMAP_FLAGS 0x22UL
#else
#error "unsupported host platform for pa24 no-host runtime helpers"
#endif

static long cppgm_test_syscall3(unsigned long number,
                                unsigned long arg0,
                                unsigned long arg1,
                                unsigned long arg2)
{
  unsigned long result = 0;
#if defined(__APPLE__)
  unsigned char carry = 0;
  __asm__ __volatile__(
      "syscall\n\t"
      "setc %1"
      : "=a"(result), "=q"(carry)
      : "0"(number), "D"(arg0), "S"(arg1), "d"(arg2)
      : "rcx", "r11", "memory");
  return carry ? -(long)result : (long)result;
#else
  __asm__ __volatile__(
      "syscall"
      : "=a"(result)
      : "0"(number), "D"(arg0), "S"(arg1), "d"(arg2)
      : "rcx", "r11", "memory");
  return (long)result;
#endif
}

static long cppgm_test_syscall6(unsigned long number,
                                unsigned long arg0,
                                unsigned long arg1,
                                unsigned long arg2,
                                unsigned long arg3,
                                unsigned long arg4,
                                unsigned long arg5)
{
  unsigned long result = 0;
  register unsigned long r10 __asm__("r10") = arg3;
  register unsigned long r8 __asm__("r8") = arg4;
  register unsigned long r9 __asm__("r9") = arg5;
#if defined(__APPLE__)
  unsigned char carry = 0;
  __asm__ __volatile__(
      "syscall\n\t"
      "setc %1"
      : "=a"(result), "=q"(carry), "+r"(r10), "+r"(r8), "+r"(r9)
      : "0"(number), "D"(arg0), "S"(arg1), "d"(arg2)
      : "rcx", "r11", "memory");
  return carry ? -(long)result : (long)result;
#else
  __asm__ __volatile__(
      "syscall"
      : "=a"(result), "+r"(r10), "+r"(r8), "+r"(r9)
      : "0"(number), "D"(arg0), "S"(arg1), "d"(arg2)
      : "rcx", "r11", "memory");
  return (long)result;
#endif
}

long cppgm_test_read_stdin(char * buf, unsigned long count)
{
  return cppgm_test_syscall3(CPPGM_TEST_SYSCALL_READ,
                             0UL,
                             (unsigned long)buf,
                             count);
}

long cppgm_test_read_all_stdin(char * buf, unsigned long count)
{
  unsigned long used = 0;
  while(used < count)
  {
    long step = cppgm_test_read_stdin(buf + used, count - used);
    if(step < 0)
    {
      return step;
    }
    if(step == 0)
    {
      return (long)used;
    }
    used += (unsigned long)step;
  }
  return -1;
}

long cppgm_test_write_stdout(const char * buf, unsigned long count)
{
  unsigned long written = 0;
  while(written < count)
  {
    long step = cppgm_test_syscall3(CPPGM_TEST_SYSCALL_WRITE,
                                    1UL,
                                    (unsigned long)(buf + written),
                                    count - written);
    if(step <= 0)
    {
      return step < 0 ? step : -1;
    }
    written += (unsigned long)step;
  }
  return (long)written;
}

int cppgm_test_write_all_stdout(const char * buf, unsigned long count)
{
  return cppgm_test_write_stdout(buf, count) < 0 ? 1 : 0;
}

void * cppgm_test_alloc(unsigned long size)
{
  long result = cppgm_test_syscall6(CPPGM_TEST_SYSCALL_MMAP,
                                    0UL,
                                    size,
                                    3UL,
                                    CPPGM_TEST_MMAP_FLAGS,
                                    (unsigned long)-1,
                                    0UL);
  return result < 0 ? 0 : (void *)result;
}
