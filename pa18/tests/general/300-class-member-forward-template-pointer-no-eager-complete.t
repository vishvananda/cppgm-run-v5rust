template<class T> struct B;
template<class T>
struct A {
  B<T>* tie();
};
template<class T>
struct B : A<T> {};
B<int> b;
