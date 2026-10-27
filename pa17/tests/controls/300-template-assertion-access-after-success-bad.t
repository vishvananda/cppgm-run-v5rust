// N3485 11.3p1: friendship to one specialization does not grant another
// specialization access, even when the assertion's value is fixed.
// expect reject
template<int> struct Owner;
class Secret {
    static constexpr bool allowed() { return true; }
    friend struct Owner<1>;
};
template<int N> struct Owner {
    static_assert(Secret::allowed(), "access still applies");
};
Owner<1> good;
Owner<2> bad;
int main() { return 0; }
