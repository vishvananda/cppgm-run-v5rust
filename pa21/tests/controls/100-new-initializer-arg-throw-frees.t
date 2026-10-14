// N3485 5.3.4p16,p18: operator new for new T(...) ran first (as g++/clang do),
// then a nested new V[1] in the initializer throws: T's storage must be freed;
// expect exit 0
typedef decltype(sizeof(0)) size_type;
static int live;
alignas(16) static char heap[256];
void* operator new(size_type) { ++live; return heap; }
void operator delete(void*) noexcept { --live; }
void* operator new[](size_type) { ++live; return heap + 64; }
void operator delete[](void*) noexcept { --live; }
struct V { V() { throw 1; } };
struct T { int v; T(int x) : v(x) {} };
int main() { try { new T((new V[1], 1)); } catch (int) {} return live; }
