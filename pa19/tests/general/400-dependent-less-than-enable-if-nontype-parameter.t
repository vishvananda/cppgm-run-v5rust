template<bool B, class T = void>
struct enable_if_impl {};

template<class T>
struct enable_if_impl<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if_impl<B, T>::type;

template<int Count, enable_if_t<Count < 2, int> = 0>
struct Box {
  static const int value = Count;
};

int main() {
  return Box<1>::value;
}
