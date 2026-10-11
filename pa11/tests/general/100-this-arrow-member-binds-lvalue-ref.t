struct A {};
void keep(A&);

struct V {
  A a;

  void f() {
    keep(this->a);
  }
};
