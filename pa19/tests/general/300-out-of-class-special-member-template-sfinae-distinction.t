template<bool, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class T>
struct input_trait {
  static const bool value = true;
};

template<class T>
struct forward_trait {
  static const bool value = true;
};

template<class Alloc>
struct box {
  template<class InputIterator, enable_if_t<input_trait<InputIterator>::value, int> = 0>
  box(InputIterator, InputIterator);

  template<class ForwardIterator, enable_if_t<forward_trait<ForwardIterator>::value, int> = 0>
  box(ForwardIterator, ForwardIterator);
};

template<class Alloc>
template<class InputIterator, enable_if_t<input_trait<InputIterator>::value, int>>
box<Alloc>::box(InputIterator, InputIterator) {}

template<class Alloc>
template<class ForwardIterator, enable_if_t<forward_trait<ForwardIterator>::value, int>>
box<Alloc>::box(ForwardIterator, ForwardIterator) {}

int main() {
  return 0;
}
