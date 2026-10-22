// N3485 5.2.1, 5.19p2, 12.2p1; CWG 1213: array prvalue elements
// belong to distinct temporaries, materialized before the subscript index.
// expect exit 0
typedef int Row[3];
typedef int Matrix[2][3];
struct Element { int value; };
typedef Element Elements[2];
struct Holder { Row values; };
typedef double Doubles[2];
constexpr int target = 11;
typedef const int* Pointers[2];

constexpr int ordered()
{
    int n = 0;
    return Row{++n, ++n, ++n}[n - 1];
}

constexpr int a = 2[(Row{4, 5, 6})];
constexpr int b = Matrix{{1, 2, 3}, {7, 8, 9}}[1][2];
constexpr int c = Holder{{12, 13, 14}}.values[1];
constexpr int d = Elements{{15}, {16}}[1].value;
constexpr double e = Doubles{1.25, 2.5}[1];
constexpr int f = *Pointers{0, &target}[1];
static_assert(a == 6 && b == 9 && c == 13 && d == 16, "subobjects");
static_assert(e == 2.5 && f == 11, "typed scalar reads");
static_assert(Row{}[2] == 0 && Row{7}[2] == 0, "zero initialization");
static_assert(ordered() == 3, "construction before index");
static_assert(&Row{1, 2, 3}[0] != &Row{1, 2, 3}[0], "distinct storage");
int main() { return a + b + c + d + f + ordered() - 58; }
