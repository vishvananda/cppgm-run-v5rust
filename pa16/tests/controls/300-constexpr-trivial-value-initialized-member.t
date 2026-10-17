// N3485 7.1.5p4, 8.5p7: trivial value-initialization runs no constructor;
// expect exit 0
struct Plain { int n; Plain() = default; };
struct Object : Plain {
    Plain first;
    Plain second;
    constexpr Object() : Plain(), first(), second{} {}
};
constexpr Object object;
static_assert(object.n == 0 && object.first.n == 0 && object.second.n == 0, "trivial value initialization");
int main() { return object.n + object.first.n + object.second.n; }
