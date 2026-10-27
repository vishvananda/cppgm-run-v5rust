// N3485 7p4: a demanded specialization with a false fixed assertion is
// ill-formed; sharing successful checks cannot suppress this failure.
// expect reject
template<int N> struct Value {
    static_assert(17 * 3 == 52, "false fixed arithmetic");
};
Value<1> bad;
int main() { return 0; }
