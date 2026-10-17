// N3485 12.8p28: an empty implicit assignment performs no subobject writes;
// expect exit 0
struct Empty {};
Empty object;
constexpr int assign() { Empty local; object = local; return 7; }
static_assert(assign() == 7, "empty subobject plan");
int main() { return assign() == 7 ? 0 : 1; }
