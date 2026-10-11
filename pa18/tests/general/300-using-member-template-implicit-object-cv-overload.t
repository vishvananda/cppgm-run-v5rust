// VALIDATION: compile-pass
// A using-imported base member remains a viable overload next to a derived
// member template. For a non-const object, overload ranking must prefer the
// non-const imported member over a const fallback template.

struct base
{
  char f();
};

struct fallback_result
{
  char payload[2];
};

struct direct_probe : base
{
  using base::f;

  template<class... Args>
  fallback_result f(Args...) const;
};

direct_probe make_direct_probe();

static_assert(sizeof(make_direct_probe().f()) == 1,
              "using-imported non-template member should be selected");

typedef char yes_type;
struct no_type
{
  char payload[2];
};

struct private_type
{
  private_type const & operator,(int) const;
};

template<class T>
no_type is_private_type(T const &);

yes_type is_private_type(private_type const &);

template<class T>
T && declval();

struct functor
{
  void func();
};

template<class Fun>
struct fun_wrap : Fun
{
  using Fun::func;

  template<class... DontCares>
  private_type func(DontCares...) const;
};

template<class Fun>
struct callable
{
  static const bool value =
      sizeof(no_type) ==
      sizeof(is_private_type((declval<fun_wrap<Fun> >().func(), 0)));
};

static_assert(callable<functor>::value,
              "SFINAE fallback should not hide the imported member");

int main()
{
  int check[callable<functor>::value ? 1 : -1];
  (void)check;
  return sizeof(make_direct_probe().f()) == 1 ? 0 : 1;
}
