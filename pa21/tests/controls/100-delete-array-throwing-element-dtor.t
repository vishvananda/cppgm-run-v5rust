// N3485 5.3.5p7, 15.2p2: when an element destructor throws during delete[],
// the remaining elements are destroyed and the deallocation function is still
// called (trace 1 1 1 7 then handler 5); expect exit 0
typedef decltype(sizeof 0) size_t;
int h, cd;
char pool[256];
struct X {
  int x;
  ~X() noexcept(false) { h = h * 10 + 1; if (--cd == 0) throw 5; }
  static void* operator new[](size_t n) { return pool; }
  static void operator delete[](void* p) { h = h * 10 + 7; }
};
int main() { X* a = new X[3]; cd = 2; try { delete[] a; } catch (int e) { h = h * 10 + e; } return h == 11175 ? 0 : 1; }
