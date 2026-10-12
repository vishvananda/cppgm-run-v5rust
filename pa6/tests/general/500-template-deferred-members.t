struct Concrete {};
struct Outer { template<class T> void f() { Concrete(); static_assert(false, "deferred"); } };
