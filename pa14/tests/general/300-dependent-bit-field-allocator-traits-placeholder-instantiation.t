template<class A>
struct allocator_traits {
  using size_type = typename A::size_type;
};

template<class A>
struct box {
  using allocator_type = A;
  using __alloc_traits = allocator_traits<allocator_type>;
  using size_type = typename __alloc_traits::size_type;

  struct inner {
    size_type cap : sizeof(size_type) * 8 - 1;
    size_type is_long : 1;
  };

  void f();
};

template<class A>
void box<A>::f() {}

int main() {
  return 0;
}
