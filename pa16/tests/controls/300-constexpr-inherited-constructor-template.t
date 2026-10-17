// N3485 12.9 and 14.8.2: inherited constructor specializations have independent forwarding forms.
// expect exit 0
struct Base {
    int n;
    template<class T> constexpr Base(T n, int extra = sizeof(T)) : n(n + extra) {}
};
struct First : Base { using Base::Base; };
struct Second : Base { using Base::Base; };
struct Indirect : First { using First::First; };
constexpr First first(3), repeated(5), full(7, 9);
constexpr Second second(11);
constexpr Indirect indirect(13);
static_assert(first.n == 7 && repeated.n == 9 && full.n == 16, "argument count and reuse");
static_assert(second.n == 15 && indirect.n == 17, "class identity and forwarding chain");
int main() {
    First runtime(3);
    return runtime.n == 7 && first.n == 7 && full.n == 16 && indirect.n == 17 ? 0 : 1;
}
