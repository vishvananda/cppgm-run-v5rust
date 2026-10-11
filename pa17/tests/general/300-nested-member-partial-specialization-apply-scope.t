// Reduced from Boost.FunctionTypes interpreter_example. A qualified
// out-of-class partial specialization of a nested class template must keep
// the nested class owner when its member template body is instantiated.

namespace example {
class interpreter {
  class token_parser {};
  class first {};
  class last {};

  template<class Function, class From = first, class To = last>
  struct invoker;

public:
  template<class Function>
  void register_function(Function f);
};

template<class Function, class From, class To>
struct interpreter::invoker {
  template<class Args>
  static inline void apply(Function f, token_parser & p, Args const & args)
  {
    typedef To next_iter_type;
    interpreter::invoker<Function, next_iter_type, To>::apply(f, p, args);
  }
};

template<class Function, class To>
struct interpreter::invoker<Function, To, To> {
  template<class Args>
  static inline void apply(Function, token_parser &, Args const &) {}
};

template<class Function>
void interpreter::register_function(Function f)
{
  void (*p)(Function, token_parser &, int const &) =
      &invoker<Function>::template apply<int>;
  token_parser state;
  p(f, state, 0);
}
}

void target() {}

int main()
{
  example::interpreter value;
  value.register_function<void (*)()>(&target);
  return 0;
}
