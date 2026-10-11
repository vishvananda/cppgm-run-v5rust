namespace outer {
namespace impl {

template<class T>
int print(T const &)
{
  return 1;
}

struct printer {
  template<class R>
  int operator()(R const & value) const
  {
    return print(value);
  }
};

}

template<class T>
struct static_const {
  static T const value;
};

template<class T>
T const static_const<T>::value = T();

namespace {
  static impl::printer const & print = static_const<impl::printer>::value;
}

template<class T>
int log(T const & value)
{
  using outer::print;
  return print(value);
}

}

struct token {};

int main()
{
  return outer::log(token()) == 1 ? 0 : 1;
}
