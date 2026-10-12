static const unsigned long left = 1, *right(2);
struct C {
    auto f(int value = 1) const & noexcept(value) -> int;
    inline C() : member(2) {}
    int member;
};
enum E { red, blue = 3 };
template<class T> struct Named { typename T::type *p; };
