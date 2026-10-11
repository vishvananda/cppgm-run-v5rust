template<class Alloc>
struct Traits {
  template<class T>
  static void destroy(Alloc&, T*) {}
};

template<class T>
struct O {
  using alloc_type = T;
  using node_traits = Traits<alloc_type>;
  alloc_type a;

  void g(T* p) { node_traits::destroy(a, p); }
};

int f() {
  struct Candidate { int x; };
  O<Candidate> o;
  Candidate c{1};
  o.g(&c);
  return 0;
}

int main() { return f(); }
