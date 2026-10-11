// N3485 focus: 7.2 [dcl.enum] opaque enum declarations
enum class FY;
FY *p;
int arr[sizeof(FY)];
static_assert(sizeof(FY) == 4, "ok");
