// N3485 focus: 7.3.4 [namespace.udir], 3.4.1 [basic.lookup.unqual]
namespace N {
  const int K = 3;
  int f();
  enum EY { A = 2 };
}
using namespace N;
static_assert(K == 3, "ok");
int arr[A];
decltype(f) *pf;
