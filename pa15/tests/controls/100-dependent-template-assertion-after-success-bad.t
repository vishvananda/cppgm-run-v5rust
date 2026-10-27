// N3485 7p4 and 14.6.2.3: an earlier successful specialization does not
// establish a dependent assertion for a different argument.
// expect reject
template<int N> struct Value {
    static_assert(17 * 3 == 51, "fixed arithmetic");
    static_assert(N > 0, "dependent value");
};
Value<1> good;
Value<-1> bad;
int main() { return 0; }
