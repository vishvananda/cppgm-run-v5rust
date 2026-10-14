// expect exit 0
void operator delete(void*, void* __param0) noexcept {}
void operator delete[](void*, void*) noexcept {}
void* operator new(unsigned long, void* pointer) { return pointer; }
void* operator new[](unsigned long, void* pointer) { return pointer; }
unsigned long long operator""_symbol(unsigned long long value) { return value + 1; }
enum E { zero = 0 };
E operator+(E value) { return value; }
E operator+(E, E) { return zero; }
bool operator&&(E, E) { return false; }
bool operator and(E, E);
int unnamed(int, int __param0, int) { return __param0; }
int main() { return unnamed(4, 0, 9); }
