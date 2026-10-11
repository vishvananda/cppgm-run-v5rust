namespace N { struct CY { enum class EY { A = 1 }; }; }
int arr[sizeof(N::CY::EY)];
static_assert(sizeof(N::CY::EY) == 4, "ok");
