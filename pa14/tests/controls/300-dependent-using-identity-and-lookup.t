// N3485 7.3.3, 14.6.2: dependent imports retain their declaring entity,
// overloads and type/value spaces; unqualified lookup skips dependent bases.
// expect exit 0
int choose() { return 1; }
struct Fixed { int choose() { return 2; } };
template<class T> struct Base {
  typedef T type;
  enum { width = sizeof(T) };
  int choose() { return 3; }
  int f(int) { return 4; }
  int f(double) { return 5; }
};
template<class T> struct Derived : Base<T> {
  using typename Base<T>::type;
  using Base<T>::width;
  using Base<T>::f;
  int f(int) { return 6; }
  type value;
  enum { extent = width + 1 };
  int direct() { return choose(); }
  int qualified() { return this->choose(); }
  struct Nested : Fixed { int call() { return choose(); } };
};
template<class T> int local() {
  struct L : T { int call() { return choose(); } };
  L value;
  return value.call();
}
struct Slot { int n; };
template<class T> struct Assign : T { using T::operator=; };
int main() {
  Derived<int> d;
  d.value = 7;
  Derived<int>::Nested nested;
  Slot source;
  source.n = 8;
  Assign<Slot> target;
  target = source;
  return d.value == 7 && d.extent == sizeof(int) + 1 &&
         d.f(1) == 6 && d.f(1.0) == 5 && d.direct() == 1 &&
         d.qualified() == 3 && nested.call() == 2 &&
         local<Base<char> >() == 1 && target.n == 8 ? 0 : 1;
}
