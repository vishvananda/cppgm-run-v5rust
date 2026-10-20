// An indirect virtual base contributes to the destructor's exception set,
// even when the intervening destructor explicitly cannot throw; expect exit 0
struct V { ~V() noexcept(false) {} };
struct A : virtual V { ~A() noexcept {} };
struct D : A {};
struct Written : A { ~Written() {} };
static_assert(!noexcept(((D*)0)->~D()), "indirect base may throw");
static_assert(!noexcept(((Written*)0)->~Written()), "written implicit specification");
void destroy_implicit(D* object) { object->~D(); }
void destroy_written(Written* object) { object->~Written(); }
int main() { return 0; }
