// N3485 5.9p2: ordering follows array elements and members with one access control.
// expect exit 0
struct Pair { int a, b; };
struct Object : Pair { Pair values[3]; int last; };
Object object;
class Private {
    int a, b;
public:
    constexpr Private() : a(1), b(2) {}
    constexpr const int* first() const { return &a; }
    constexpr const int* second() const { return &b; }
};
constexpr Private private_object;
static_assert(&object.a < &object.b, "members of one base");
static_assert(&object.values[0].b < &object.values[2].a, "array elements");
static_assert(&object.values[1].a < &object.values[1].b, "members in one array element");
static_assert(&object.values[2].b < &object.last, "members of the complete object");
static_assert(private_object.first() < private_object.second(), "same private access");
int main() { return &object.values[1].a < &object.values[1].b ? 0 : 1; }
