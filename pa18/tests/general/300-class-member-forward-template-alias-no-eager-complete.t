template<class T> struct B;
template<class T>
struct A {
  using other_type = B<T>;
  other_type* tie();
};
template<class T>
struct B : A<T> {};
B<int> b;
