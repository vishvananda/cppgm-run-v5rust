template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class Rep>
struct box {
  typedef Rep rep;

  rep value;

  template<class Rep2, enable_if_t<__is_convertible(const Rep2&, rep), int> = 0>
  explicit box(const Rep2& r) : value(r) {}
};

int main() {
  int x = 0;
  int* p = &x;
  int** pp = &p;
  box<int* const*> b(pp);
  return b.value == 0;
}
