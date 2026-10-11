template<class T, class Alloc>
struct sequence;

template<class Alloc>
struct sequence<bool, Alloc> {
  using allocator_type = Alloc;

  sequence(const allocator_type&);
};

template<class Alloc>
sequence<bool, Alloc>::sequence(
    const sequence<bool, Alloc>::allocator_type&) {}

int main() { return 0; }
