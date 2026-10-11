// VALIDATION: compile-pass
// Every operand in a defaulted decltype pack must still be checked for SFINAE.
template<class T> T && value();
template<class...> struct types {};

template<class F> struct call {
  template<class... T,
           class = types<decltype(value<F>()(value<T>()))...> >
  void operator()(T &&...) const;
};

template<class F> char probe(decltype(value<F>()(value<int>())) *);
template<class> long probe(...);

struct no_call {};
static_assert(sizeof(probe<call<no_call> >(0)) == sizeof(long), "");
int main() {}
