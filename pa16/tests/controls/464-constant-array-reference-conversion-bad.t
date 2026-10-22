// Frozen interpreter boundary: bind() materializes an array prvalue, then
// convert_scalar rejects its array value. Preserve that rejection.
// expect reject
typedef int Pair[2];
constexpr int first(const Pair& value) { return value[0]; }
constexpr int result = first(Pair{3, 4});
int main() { return result; }
