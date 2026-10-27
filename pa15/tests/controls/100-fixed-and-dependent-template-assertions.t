// N3485 7p4, 14.6p8 and 14.6.2.3: fixed conditions keep their meaning
// across specializations, while dependent conditions use each argument.
// expect exit 0
template<int N> struct Value {
    static_assert(17 * 3 == 51, "fixed arithmetic");
    static_assert(sizeof(int) >= 2, "fixed type");
    static_assert(N > 0, "dependent value");
    int get() {
        static_assert(9 + 4 == 13, "fixed function condition");
        static_assert(N < 10, "dependent function condition");
        return N;
    }
};
int main() {
    Value<1> a;
    Value<4> b;
    return a.get() + b.get() == 5 ? 0 : 1;
}
