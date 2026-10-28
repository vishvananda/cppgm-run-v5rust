namespace std { template<class T> class initializer_list; }
struct A { A(std::initializer_list<int>); ~A(); };
void f() {
  std::initializer_list<A> a = {std::initializer_list<int>{1, 2}};
}
void persistent() {
  static std::initializer_list<A> a = {std::initializer_list<int>{3, 4}};
}
