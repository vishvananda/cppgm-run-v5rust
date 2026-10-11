int which;

template<class T>
struct A {
  A(A const&) { which = 1; }
  A(A&&) { which = 2; }

  template<class U>
  A(A<U> const&) { which = 3; }

  template<class U>
  A(A<U>&&) { which = 4; }

  ~A() {}
};

A<int> h();

int g() {
  A<int> a = h();
  return which;
}
