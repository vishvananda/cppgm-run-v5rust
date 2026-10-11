template<class T>
struct allocator {
  template<class U> using rebind = allocator<U>;
};

template<class Alloc, class T>
using rebound_alias = typename Alloc::template rebind<T>;

typedef rebound_alias<allocator<int>, char> rebound_t;
rebound_t x;

int main() { return 0; }
