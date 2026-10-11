template <class Derived, class T>
struct layout {
  int __back_spare() const { return 7; }
};

template <class T>
struct split : layout<split<T>, T> {
  using __base_type = layout<split<T>, T>;
  using __base_type::__back_spare;

  int spare() const { return __back_spare(); }
};

int main() {
  split<int> s;
  return s.spare() - 7;
}
