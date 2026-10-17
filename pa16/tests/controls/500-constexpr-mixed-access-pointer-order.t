// N3485 5.9p2: members with different access controls have no specified order.
// expect reject
struct Object {
private:
    int first;
public:
    int second;
    constexpr Object() : first(1), second(2) {}
    constexpr const int* address() const { return &first; }
};
constexpr Object object;
static_assert(object.address() < &object.second, "unspecified comparison");
