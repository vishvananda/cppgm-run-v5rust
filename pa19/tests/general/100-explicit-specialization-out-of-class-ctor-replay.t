template<class T>
struct S {
  int v;
  S();
  S(int x);
};

template<class T>
S<T>::S() : v(1) {}

template<class T>
S<T>::S(int x) : v(x) {}

template<>
struct S<int> {
  int v;
  S();
  S(int x);
};

S<int>::S() : v(2) {}

S<int>::S(int x) : v(x + 10) {}

int f() {
  S<int> b;
  S<int> c(5);
  return b.v + c.v;
}

int main() { return f(); }
