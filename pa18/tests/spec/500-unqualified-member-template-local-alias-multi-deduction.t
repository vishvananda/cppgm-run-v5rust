// VALIDATION: compile-pass
// N3485 focus: 14.8.2.1 [temp.deduct.call]

struct state
{
};

template<class T>
struct traits
{
};

namespace detail
{
template<class, bool, bool>
struct matcher;

template<class T, bool B1, bool B2>
struct matcher
{
};

template<class T>
class compiler
{
public:
  bool run();

private:
  template<bool B1, bool B2>
  void insert();

  template<bool B1, bool B2>
  using matcher = ::detail::matcher<T, B1, B2>;

  template<bool B1, bool B2>
  bool term(state &, matcher<B1, B2> &);

};

template<class T>
bool compiler<T>::run()
{
  insert<true, false>();
  return false;
}

template<class T>
template<bool B1, bool B2>
void compiler<T>::insert()
{
  state s;
  matcher<B1, B2> m;
  while (term(s, m))
    ;
}

template<class T>
template<bool B1, bool B2>
bool compiler<T>::term(state &, matcher<B1, B2> &)
{
  return B1 && B2;
}
}

detail::compiler<traits<char> > *get_compiler();

int main()
{
  return get_compiler()->run() ? 1 : 0;
}
