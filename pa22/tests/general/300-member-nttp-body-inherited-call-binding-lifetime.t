struct B { void f() const; };
template<class C, void (C::*)()> struct R {};
struct D : B {
  void g() { f(); }
  R<D, &D::g> r;
};
