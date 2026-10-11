template<class T>
struct allocator {
  template<class U> struct rebind { typedef allocator<U> type; };
};

template<class Alloc>
using rebound_alias = typename Alloc::template rebind<char>::type;

typedef rebound_alias<allocator<int> > rebound_t;
rebound_t x;

int main() { return 0; }
