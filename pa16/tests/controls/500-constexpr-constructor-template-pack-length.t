// N3485 14.5.3p5: packs expanded together require equal lengths.
// expect reject
struct First { constexpr First(int) {} };
struct Second { constexpr Second(int) {} };
template<class... T> struct Derived : T... {
    template<class... U> constexpr Derived(U... value) : T(value)... {}
};
constexpr Derived<First, Second> object(First(3));
