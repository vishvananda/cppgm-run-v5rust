// Demand the clock values without printing their volatile spellings.
#define FORWARD(x) _Pragma(x)
FORWARD(__DATE__)
FORWARD(__TIME__)
#if !defined(__DATE__) || !defined(__TIME__) || !defined(__CPPGM_AUTHOR__)
#error missing predefined values
#endif
17
