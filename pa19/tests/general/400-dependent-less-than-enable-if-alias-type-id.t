template<bool B, class T = void>
struct enable_if_impl {};

template<class T>
struct enable_if_impl<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if_impl<B, T>::type;

template<int Count>
using selected_t = enable_if_t<Count < 2, int>;

selected_t<1> x = 7;

int main() {
  return x;
}
