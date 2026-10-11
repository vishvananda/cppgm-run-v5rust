// VALIDATION: compile-pass
template<class...> class A;
template<int, class... T> int& get(A<T...>&);
template<class... T> class A {
  int value;
  template<int, class... U> friend int& get(A<U...>&);
};
template<int I, class... T> int& get(A<T...>& a) { return a.value; }
int main() { A<int> a; return get<0>(a); }
