template<class T> struct A {
  struct B { template<class U> struct C; };
};
template<class T> template<class U>
struct A<T>::B::C<A<T>::B::C<int>> {};
