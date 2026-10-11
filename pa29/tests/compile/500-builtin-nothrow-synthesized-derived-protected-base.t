template<class> struct B { protected: B() noexcept {} };
struct D : B<D> {};
static_assert(__is_nothrow_constructible(D), "");
int main() {}
