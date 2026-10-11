// Reduced from Boost.FunctionTypes interpreter_example. A non-template class
// may have its reference members collected before full collection; an
// out-of-class nested class template definition must survive that reset.

namespace meta {
template<class F> struct begin { typedef int type; };
template<class F> struct end { typedef long type; };
template<class T> struct next { typedef long type; };
}

class interpreter {
  template<class Function,
           class From = typename meta::begin<Function>::type,
           class To = typename meta::end<Function>::type>
  struct invoker;

public:
  template<class Function>
  void register_function(Function f);
};

template<class Function, class From, class To>
struct interpreter::invoker {
  template<class Args>
  static void apply(Function f, Args const & args)
  {
    interpreter::invoker<Function, typename meta::next<From>::type, To>::apply(f, args);
  }
};

template<class Function, class To>
struct interpreter::invoker<Function, To, To> {
  template<class Args>
  static void apply(Function, Args const &) {}
};

template<class Function>
void interpreter::register_function(Function f)
{
  void (*p)(Function, int const &) = &invoker<Function>::template apply<int>;
  p(f, 0);
}

void target() {}

int main()
{
  interpreter value;
  value.register_function<void (*)()>(&target);
  return 0;
}
