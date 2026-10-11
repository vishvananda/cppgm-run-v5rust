template<class A>
struct G {
  typedef unsigned long size_type;

  template<class B>
  G(B b, size_type n) {}
};

int main() {
  int *p = 0;
  G<int*> g(p, 1);
  return 0;
}
