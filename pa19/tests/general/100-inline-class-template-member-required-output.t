template <class P, class S = unsigned long>
struct allocation_result {
  P ptr;
  S count;
};

template <class T>
struct allocator {
  using pointer = T*;

  constexpr allocation_result<T*> allocate_at_least(unsigned long n) {
    return {0, n};
  }
};

template <class Alloc>
struct allocator_traits {
  using pointer = typename Alloc::pointer;
  using size_type = unsigned long;

  static constexpr allocation_result<pointer, size_type>
  allocate_at_least(Alloc& a, size_type n) {
    return a.allocate_at_least(n);
  }
};

int main() {
  allocator<int> a;
  allocation_result<int*> result =
      allocator_traits<allocator<int>>::allocate_at_least(a, 1);
  return result.count == 1 ? 0 : 1;
}
