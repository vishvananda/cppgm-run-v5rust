// The host compiler's intrinsic headers compile in hosted mode and their
// SSE/SSE2 operations, lane extraction and insertion, _mm_malloc and
// <x86intrin.h>'s byte swap and bit scan run as the host compiler's build
// does. GCC's intrinsics are gnu_inline always_inline functions emitted only
// where used: an unused one's asm (pconfigintrin.h's pconfig) is never
// assembled, and at -O1 and above a lane builtin's selector is the wrapper's
// parameter, a constant once the wrapper is inlined.
