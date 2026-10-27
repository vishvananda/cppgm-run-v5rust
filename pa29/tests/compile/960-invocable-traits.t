struct C { int field; int call(int) noexcept; };
struct Wrapper { C& operator*() const noexcept; };
struct Functor { int operator()(int) const noexcept; };
struct Hidden { private: int operator()(int) const; };
struct Dormant {
  template<class T> int operator()(T) const noexcept { T::missing(); }
};
struct Incomplete;
typedef int (*Function)(int);

static_assert(__is_invocable(Function, int), "function pointer");
static_assert(!__is_nothrow_invocable(Function, int), "indirect call");
static_assert(__is_nothrow_invocable(Functor, int), "nothrow functor");
static_assert(__is_nothrow_invocable(Dormant, int), "signature only");
static_assert(__is_invocable(int C::*, C&), "member object");
static_assert(__is_nothrow_invocable(int C::*, Wrapper), "indirection");
static_assert(!__is_invocable(Hidden, int), "private call");
static_assert(!__is_invocable(int C::*, Incomplete, int), "arity first");
static_assert(!__is_invocable(void, int), "void callable");

int main() { return 0; }
