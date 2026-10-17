// N3485 5.16p6, 12.8p31: the selected operand retains its copy initialization;
// expect exit 0
struct Object {
    int n;
    constexpr Object(int n) : n(n) {}
    constexpr Object(const Object& other) : n(other.n + 1) {}
};
constexpr Object original(3);
constexpr Object select(bool first) { return first ? original : Object(8); }
constexpr Object copied = select(true);
constexpr Object direct = select(false);
static_assert(copied.n == 4 && direct.n == 8, "selected copy versus direct construction");
int main() { return copied.n == 4 && original.n == 3 ? 0 : 1; }
