template<class C, class B> struct local {};
template<class M> struct collection {
  template<class B> using local = ::local<collection, B>;
  template<class T> static local<T*> select(local<const T*>) { return {}; }
};
int main() {
  collection<int>::select(::local<collection<int>, const int*>());
}
