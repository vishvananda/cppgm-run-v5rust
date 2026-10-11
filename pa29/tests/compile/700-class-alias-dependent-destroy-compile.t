template<bool B, class T = int>
struct EnableIf {};

template<class T>
struct EnableIf<true, T> {
  using type = T;
};

template<class Alloc, class P>
struct HasDestroy {
  static constexpr bool value = false;
};

template<class T>
struct ValueAlloc {
  void destroy(T*) {}
};

template<class T>
struct NodeAlloc {
  void destroy(T*) {}
};

template<class T>
struct NodeBaseAlloc {
  void destroy(T*) {}
};

template<class T>
struct HasDestroy<ValueAlloc<T>, T*> {
  static constexpr bool value = true;
};

template<class T>
struct HasDestroy<NodeAlloc<T>, T*> {
  static constexpr bool value = true;
};

template<class T>
struct HasDestroy<NodeBaseAlloc<T>, T*> {
  static constexpr bool value = true;
};

template<class Alloc>
struct Traits {
  using allocator_type = Alloc;

  template<class T,
           typename EnableIf<HasDestroy<allocator_type, T*>::value, int>::type = 0>
  static void destroy(Alloc& a, T* p) { a.destroy(p); }

  template<class T,
           typename EnableIf<!HasDestroy<allocator_type, T*>::value, int>::type = 0>
  static void destroy(Alloc&, T*) {}
};

int f() {
  struct Candidate { int x; };
  Candidate c{1};
  ValueAlloc<Candidate> value_alloc;
  NodeAlloc<Candidate> node_alloc;
  NodeBaseAlloc<Candidate> node_base_alloc;
  Traits<ValueAlloc<Candidate>>::destroy(value_alloc, &c);
  Traits<NodeAlloc<Candidate>>::destroy(node_alloc, &c);
  Traits<NodeBaseAlloc<Candidate>>::destroy(node_base_alloc, &c);
  return c.x;
}

int main() { return f(); }
