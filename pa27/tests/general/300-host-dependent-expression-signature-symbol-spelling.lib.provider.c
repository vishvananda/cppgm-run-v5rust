/* Calls the course unit's instantiations by the names clang gives them, so
   the link holds only if the course spells each one as clang does. */
#if defined(__APPLE__)
#define SYM(name) "_" name
#else
#define SYM(name) name
#endif

struct runner { char unused; };

int measure_long(long *value)
  __asm__(SYM("_Z7measureIlEDTcldtcv6holderIT_E_E3getEERS1_"));
int wide_int(int value)
  __asm__(SYM("_Z4wideIiEN9enable_ifIXaagtstT_Li2Entsr10is_pointerIS1_EE5valueEiE4typeES1_"));
int fill_short(short *first, int (*counts)[3])
  __asm__(SYM("_Z4fillIsEiPT_RAplstS0_Li1E_i"));
long checked_long(long value)
  __asm__(SYM("_Z7checkedIlEN9enable_ifILb1ET_E4typeES1_"));
int ran_runner(struct runner *value)
  __asm__(SYM("_Z3ranI6runnerEDTcmcldtcl7declvalIT_EE3runEcvi_EERS1_"));

int host_calls(void) __asm__(SYM("_Z10host_callsv"));

int host_calls(void)
{
  long l = 1;
  short s = 9;
  int counts[3] = { 0, 0, 0 };
  struct runner r;
  int sum = measure_long(&l) + wide_int(5) + fill_short(&s, &counts);
  return sum + counts[0] + (int)checked_long(41L) + ran_runner(&r);
}
