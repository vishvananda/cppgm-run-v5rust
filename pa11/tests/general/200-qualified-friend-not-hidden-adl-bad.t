// VALIDATION: compile-fail

namespace ns {

struct X;

namespace detail {
void f(X const &);
}

struct Base {
  friend void detail::f(X const &);
};

struct X : Base {};

namespace detail {
void f(X const &) {}
}

void g(X const & x)
{
  f(x);
}

}

int main()
{
  ns::X x;
  ns::g(x);
  return 0;
}
