// N3485 17.6.4.6p2, 18.6.1.1, 18.6.1.2: replacing only operator new[] and
// operator delete[] leaves operator new and operator delete the
// implementation's: a scalar new-expression never calls the program's array
// form. In the program's LowIR (pa8/lowir.md) each singleton role has one
// owner: the implementation's scalar forms own allocate_memory and
// free_memory, and the replaced array forms have no role. expect exit 0
typedef decltype(sizeof(0)) size_type;
alignas(16) static char pool[256];
static size_type used;
static int arrays;
void* operator new[](size_type n) { ++arrays; void* p = pool + used; used += (n + 15) / 16 * 16; return p; }
void operator delete[](void* p) noexcept { if (p) --arrays; }
struct E { E() { throw 2; } };
int main() {
  int* p = new int(5);
  if (arrays != 0) return 1;
  int* q = new int[3];
  if (arrays != 1) return 2;
  delete p;
  delete[] q;
  if (arrays != 0) return 3;
  try { new E[2]; return 4; } catch (int) {}
  if (arrays != 0) return 5;
  int* r = new int(7);
  int ok = *r == 7 && arrays == 0;
  delete r;
  return ok ? 0 : 6;
}
