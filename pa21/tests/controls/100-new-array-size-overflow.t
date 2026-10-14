// N3485 5.3.4p7: if count*sizeof(T) exceeds the implementation limit the
// new-expression throws bad_array_new_length (g++) and never passes a wrapped
// size (clang passes SIZE_MAX); expect exit 0
static unsigned long requested = 1;
void* operator new[](unsigned long n) { requested = n; static char buf[64]; return buf; }
void operator delete[](void*) noexcept {}
volatile unsigned long n = 0x4000000000000001ul;
int main() { try { new int[n]; } catch (...) { return 0; } return requested == ~0ul ? 0 : 1; }
