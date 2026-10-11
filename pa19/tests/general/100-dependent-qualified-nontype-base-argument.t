template<bool B>
struct bool_constant {
  static const bool value = B;
};

template<class T, bool Cached>
struct Node {
  T value;
};

template<class Alloc, class T>
struct Rebind {
  typedef T value_type;
};

template<class T>
struct Holder {
  Holder() {}
  T value;
};

template<class NodeAlloc>
struct AllocBox {
  Holder<NodeAlloc> holder;
};

template<class Traits>
struct Table
  : AllocBox<Rebind<int, Node<int, Traits::hash_cached::value> > > {
  typedef int size_type;
  Table(size_type);
};

template<class Traits>
Table<Traits>::Table(size_type) {}

struct Traits {
  typedef bool_constant<false> hash_cached;
};

int main()
{
  Table<Traits> table(0);
  return 0;
}
