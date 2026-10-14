// N3485 5.3.4p13: a class-specific noexcept operator new(size_t, void*)
// returning null means no initialization and a null result (only ::operator
// new(size_t, void*) is the reserved form); expect exit 0
typedef unsigned long size_t;
int ctors;
struct T { static void* operator new(size_t, void* p) noexcept { return p; } T() { ++ctors; } };
int main() { T* t = new (static_cast<void*>(0)) T; return (t == 0 && ctors == 0) ? 0 : 1; }
