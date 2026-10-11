// VALIDATION: compile-pass
// A function parameter pack followed by another function parameter pack must
// bind the concrete parameters to the pack whose template type pack is nonempty.

template<class T>
T && forward(T & value)
{
  return static_cast<T &&>(value);
}

struct tag {};

struct arg_list {
  template<class... ReversedArgs>
  arg_list(tag, ReversedArgs&&...) {}
};

template<class ArgList, class... Args>
struct arg_list_factory;

template<class ArgList>
struct arg_list_factory<ArgList> {
  template<class... ReversedArgs>
  static ArgList reverse(ReversedArgs&&... reversed_args) {
    return ArgList(tag(), forward<ReversedArgs>(reversed_args)...);
  }
};

template<class ArgList, class A0, class... Args>
struct arg_list_factory<ArgList, A0, Args...> {
  template<class... ReversedArgs>
  static ArgList reverse(A0&& a0, Args&&... args, ReversedArgs&&... reversed_args) {
    return arg_list_factory<ArgList, Args...>::reverse(
        forward<Args>(args)...,
        forward<A0>(a0),
        forward<ReversedArgs>(reversed_args)...);
  }
};

int main() {
  int a = 0;
  int b = 0;
  arg_list_factory<arg_list, int&, int&>::reverse(a, b);
  return 0;
}
