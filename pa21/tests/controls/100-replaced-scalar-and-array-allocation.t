// N3485 17.6.4.6p2, 18.6.1.1, 18.6.1.2: a program may replace operator
// new, operator new[], operator delete and operator delete[] together; each
// new-expression and delete-expression calls the program's function for
// its form, and a class's operator new (12.5) is still found first for its
// class. 5.3.4p18: when the nested new V[1] in T's initializer throws, T's
// storage is freed by the replaced operator delete. In the program's
// LowIR (pa8/lowir.md) each singleton role has one owner: the scalar forms
// own allocate_memory and free_memory, and the replaced array forms have no
// role. expect exit 0
typedef decltype(sizeof(0)) size_type;
alignas(16) static char pool[1024];
static size_type used;
static int scalars, arrays, members;
static void* take(size_type n) { void* p = pool + used; used += (n + 15) / 16 * 16; return p; }
void* operator new(size_type n) { ++scalars; return take(n); }
void operator delete(void* p) noexcept { if (p) --scalars; }
void* operator new[](size_type n) { ++arrays; return take(n); }
void operator delete[](void* p) noexcept { if (p) --arrays; }
struct V { V() { throw 1; } };
struct T { int v; T(int x) : v(x) {} };
struct M {
  int m;
  static void* operator new(size_type n) { ++members; return take(n); }
  static void operator delete(void*) { --members; }
};
int main() {
  try { new T((new V[1], 1)); return 1; } catch (int) {}
  if (scalars != 0 || arrays != 0) return 2;
  int* p = new int(4);
  int* q = new int[3];
  M* m = new M;
  M* g = ::new M;
  if (scalars != 2 || arrays != 1 || members != 1) return 3;
  ::delete g;
  delete m;
  delete[] q;
  delete p;
  return scalars == 0 && arrays == 0 && members == 0 ? 0 : 4;
}
