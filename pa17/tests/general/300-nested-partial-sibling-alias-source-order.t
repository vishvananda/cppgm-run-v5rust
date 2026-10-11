// VALIDATION: compile-pass
// A nested partial specialization resolves its own preceding alias before an
// enclosing class member with the same name.

struct operation
{
  template<class Arg>
  struct apply
  {
    typedef Arg type;
  };
};

template<class Operation>
struct function
{
  typedef void arg;

  template<class Signature>
  struct result;

  template<class Arg>
  struct result<void(Arg)>
  {
    typedef Arg arg;
    typedef typename Operation::template apply<arg>::type impl;
  };

  template<class Arg>
  int operator()(Arg const &) const
  {
    return typename result<void(Arg)>::impl();
  }
};

int main()
{
  function<operation> f;
  return f(0);
}
