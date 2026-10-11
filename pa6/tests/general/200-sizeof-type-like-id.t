enum class FY;
int arr[sizeof(FY)];
static_assert(sizeof(FY) == 4, "ok");
