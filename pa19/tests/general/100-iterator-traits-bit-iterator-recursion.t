template<class T>
struct has_iterator_typedefs {
  static const bool value = true;
};

template<class Iter, bool>
struct __iterator_traits {};

template<class Iter>
struct iterator_traits : __iterator_traits<Iter, has_iterator_typedefs<Iter>::value> {
  using difference_type = typename Iter::difference_type;
};

template<class Container, bool IsConst, typename Container::storage_type Dummy>
struct bit_iterator {
  using difference_type = int;
  using value_type = bool;
  using pointer = bit_iterator;
  using reference = bool;
  using iterator_category = int;
};

struct Container {
  using storage_type = unsigned long;
};

int main() {
  iterator_traits<bit_iterator<Container, false, 0>>::difference_type x = 0;
  return x;
}
