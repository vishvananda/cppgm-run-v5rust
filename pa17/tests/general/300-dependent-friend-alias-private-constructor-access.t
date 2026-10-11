template<class C>
class Ref {
public:
  typedef typename C::__storage_pointer P;
  typedef typename C::__storage_type M;
  Ref(const Ref&);
private:
  friend typename C::__self;
  Ref(P, M) {}
};

struct V {
  typedef V __self;
  typedef unsigned long* __storage_pointer;
  typedef unsigned long __storage_type;
  typedef Ref<V> reference;
  __storage_pointer p;
  reference make() { return reference(p, __storage_type(1)); }
};

int main() { return 0; }
