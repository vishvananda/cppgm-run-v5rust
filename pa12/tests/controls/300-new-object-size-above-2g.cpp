// N3485 5.3.4p10/5.3.3p6: operator new receives sizeof(T) as std::size_t; a
// class of 0x80000010 bytes requests exactly that (not sign-extended from 32
// bits); expect exit 0
static unsigned long requested;
alignas(16) static char heap[64];
void* operator new(unsigned long n) { requested = n; return heap; }
void operator delete(void*) noexcept {}
struct Big { char bytes[0x80000010ul]; };
int main() { Big* b = new Big; (void)b; return requested == 0x80000010ul ? 0 : 1; }
