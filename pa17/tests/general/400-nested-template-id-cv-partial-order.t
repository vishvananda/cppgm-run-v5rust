// VALIDATION: compile-pass
// A nested cv-qualified template-id must neither match its unqualified peer
// nor be discarded while ordering the two partial specializations.

template<class T>
struct actor {};

template<class T>
struct wrapper {};

template<class T>
struct custom
{
  static const int value = 0;
};

template<class Expr>
struct custom<wrapper<actor<Expr> > >
{
  static const int value = 1;
};

template<class Expr>
struct custom<wrapper<actor<Expr> const> >
{
  static const int value = 2;
};

static_assert(custom<wrapper<actor<int> > >::value == 1,
              "unqualified nested template-id partial");
static_assert(custom<wrapper<actor<int> const> >::value == 2,
              "const nested template-id partial");

int main()
{
  return 0;
}
