// N3485 3.2p2 and 5.16: a conditional reference retains both initializer arms' call demand.
// expect exit 0
int first = 3, second = 5;
constexpr bool choose() { return true; }
constexpr const int& global_first() { return first; }
constexpr const int& global_second() { return second; }
constexpr const int& local_first() { return first; }
constexpr const int& local_second() { return second; }
constexpr const int& unused() { return first; }
constexpr const int& global_reference = choose() ? global_first() : global_second();
int local() {
    static constexpr const int& reference = false ? local_first() : local_second();
    return reference;
}
struct Box {
    const int* pointer;
    constexpr Box(const int* pointer) : pointer(pointer) {}
    constexpr operator const int&() const { return *pointer; }
};
constexpr const int& converted = true ? Box(&first) : Box(&second);
static_assert(&converted == &first, "conversion returns a place without reading its value");
int main() { return global_reference == 3 && converted == 3 && local() == 5 ? 0 : 1; }
