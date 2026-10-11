template<class T, class U>
struct same
{
  static const bool value = false;
};

template<class T>
struct same<T, T>
{
  static const bool value = true;
};

struct Base
{
  int member;
};

struct Derived : Base
{
};

Base const && base_rvalue();
Base const & base_lvalue();
Base const * base_pointer();
Derived const && derived_rvalue();
Derived const & derived_lvalue();
Derived const * derived_pointer();
int Base::* member_pointer();

static_assert(same<decltype(base_rvalue().*member_pointer()), int const &&>::value,
              "base rvalue member cv");
static_assert(same<decltype(base_lvalue().*member_pointer()), int const &>::value,
              "base lvalue member cv");
static_assert(same<decltype(base_pointer()->*member_pointer()), int const &>::value,
              "base pointer member cv");
static_assert(same<decltype(derived_rvalue().*member_pointer()), int const &&>::value,
              "derived rvalue member cv");
static_assert(same<decltype(derived_lvalue().*member_pointer()), int const &>::value,
              "derived lvalue member cv");
static_assert(same<decltype(derived_pointer()->*member_pointer()), int const &>::value,
              "derived pointer member cv");

int main()
{
  return 0;
}
