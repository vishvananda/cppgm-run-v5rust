template<class T>
struct allocator {
  template<class U> using rebind = allocator<U>;
};

template<class Alloc>
using rebound_alias = typename Alloc::template rebind<char>;

typedef rebound_alias<allocator<int> > rebound_t;
rebound_t x;

int main() { return 0; }
