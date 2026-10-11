template<class C, class B> struct local {};
template<class M> struct segment {
  template<class T> using const_iterator = const T*;
};
template<class M> struct collection {
  template<class T> using const_base =
      typename segment<M>::template const_iterator<T>;
  template<class T> using iterator = local<collection, T*>;
  template<class T> using const_iterator = local<collection, const_base<T>>;
  template<class T> static iterator<T> select(const_iterator<T>) { return {}; }
};
int main() {
  collection<int>::select(local<collection<int>, const int*>());
}
