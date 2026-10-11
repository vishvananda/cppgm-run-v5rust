/* Defines the instantiations under the names clang gives them; the course
   unit only declares the templates, so its calls link only if the course
   spells each one as clang does. */
#if defined(__APPLE__)
#define SYM(name) "_" name
#else
#define SYM(name) name
#endif

struct box_int { int value; };

int unbox_int(const struct box_int *b)
  __asm__(SYM("_Z5unboxIiEDTcldtfp_3getEERK3boxIT_E"));
int twice_int(int value)
  __asm__(SYM("_Z5twiceIiEN9enable_ifIXaasr11is_integralIT_EE5valueeqstS1_Li4EES1_E4typeES1_"));
int last_short_3(short (*values)[2], int (*counts)[6])
  __asm__(SYM("_Z4lastIsLi3EEiRAmiT0_Li1E_T_RAmlstS0_T0__i"));

int unbox_int(const struct box_int *b) { return b->value + 1; }
int twice_int(int value) { return value * 2; }
int last_short_3(short (*values)[2], int (*counts)[6])
{
  (void)counts;
  return (*values)[1];
}
