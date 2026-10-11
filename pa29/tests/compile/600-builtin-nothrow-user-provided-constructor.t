struct M { M() {} };
static_assert(!__is_nothrow_constructible(M), "");
int throwing_default();
struct WithDefault { WithDefault(int = throwing_default()) noexcept {} };
static_assert(!__is_nothrow_constructible(WithDefault), "used default may throw");
static_assert(__is_nothrow_constructible(WithDefault, int), "unused default");
struct InheritedDefault : WithDefault {
  using WithDefault::WithDefault;
  InheritedDefault(InheritedDefault&&) = default;
};
static_assert(!__is_nothrow_constructible(InheritedDefault), "inherited default");
static_assert(__is_nothrow_constructible(InheritedDefault, int), "explicit argument");
int main() { return 0; }
