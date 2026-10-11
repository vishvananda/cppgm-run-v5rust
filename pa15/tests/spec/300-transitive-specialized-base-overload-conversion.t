struct B {};
template<class> struct M {};
template<> struct M<int> : B {};
template<class T> struct D : M<T> {};
template<class T> D<T>* make() { return 0; }
struct F {
  void operator()(int) {}
  void operator()(B*) {}
};
int main() { F f; f(make<int>()); }
