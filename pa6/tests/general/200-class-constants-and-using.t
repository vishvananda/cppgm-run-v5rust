struct CY { static const int N = 3; enum class EY { A = 2 }; int arr[N]; };
int arr2[CY::N];
decltype(CY::N) n;
using CY::N;
using CY::EY;
EY ey;
static_assert(N == 3, "ok");
