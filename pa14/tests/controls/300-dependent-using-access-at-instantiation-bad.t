// N3485 7.3.3p17, 14.6.2: a dependent using defers access checks until
// instantiation; it checks every overload even if an own member hides one.
// expect reject
template<class T> class Base {
  int f(int) { return 1; }
public:
  int f(double) { return 2; }
};
template<class T> struct Derived : Base<T> {
  using Base<T>::f;
  int f(int) { return 3; }
};
Derived<int> value;
