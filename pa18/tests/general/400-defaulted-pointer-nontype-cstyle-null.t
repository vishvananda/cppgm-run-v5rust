namespace detail {
struct marker;
}

template<typename T = int, detail::marker * = (detail::marker *)0>
struct holder;

template<typename T, detail::marker *>
struct holder {
  typedef T type;
};

template<typename T>
struct use_holder {
  typedef typename holder<>::type type;
};

int main() {
  use_holder<int>::type x = 0;
  return x;
}
