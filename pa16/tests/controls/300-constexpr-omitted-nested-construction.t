// N3485 8.5.1p7, 8.5p7: omitted subobjects are value-initialized;
// expect exit 0
struct Leaf { int n; constexpr Leaf() : n(7) {} };
struct Nested { Leaf leaf; int zero; };
struct Object { int first; Nested nested; Leaf array[2][2]; };
constexpr Object object = {3};
static_assert(object.nested.leaf.n == 7 && object.nested.zero == 0, "nested omitted aggregate");
static_assert(object.array[0][0].n == 7 && object.array[1][1].n == 7, "omitted class array");
constexpr int local() { Object value = {4}; return value.nested.leaf.n + value.array[1][0].n; }
static_assert(local() == 14, "evaluation-owned nested construction");
int main() { return object.array[1][1].n == 7 ? 0 : 1; }
