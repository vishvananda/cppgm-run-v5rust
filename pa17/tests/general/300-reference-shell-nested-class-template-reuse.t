template<class T>
struct allocator {
  template<class U>
  struct rebind { typedef allocator<U> other; };

  int value;
};

template<class A>
struct use {
  typedef typename A::template rebind<char>::other rebound;
};

typedef use<allocator<int> >::rebound rebound_t;

int main() {
  rebound_t x;
  x.value = 7;
  return x.value;
}
