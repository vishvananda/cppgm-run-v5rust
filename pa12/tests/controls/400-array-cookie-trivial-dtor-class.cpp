// Itanium ABI 2.7: new P[n] for a class with a trivial destructor (and usual
// operator delete[](void*)) has no cookie: operator new[] receives exactly
// n*sizeof(P); expect exit 0
static unsigned long requested;
alignas(16) static char heap[256];
void* operator new[](unsigned long n) { requested = n; return heap; }
void operator delete[](void*) noexcept {}
struct P { int a, b; };
int main() { volatile int n = 3; P* p = new P[n]; delete[] p; return requested == 3 * sizeof(P) ? 0 : 1; }
