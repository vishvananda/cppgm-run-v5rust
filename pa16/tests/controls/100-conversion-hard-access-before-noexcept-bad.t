// expect reject
// A hard conversion access error precedes demand of its noexcept specification.
template<class T> struct A {
private:
  operator int() noexcept(sizeof(typename T::missing) > 0);
};
int convert(A<int>& a) { return a; }
