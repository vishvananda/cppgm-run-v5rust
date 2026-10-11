namespace N {
  const int K = 5;
  int f();
  enum EY { A = 7 };
}
using N::K;
using N::K;
using N::f;
using N::A;
static_assert(K == 5, "ok");
int arr[A];
decltype(f) *pf;
