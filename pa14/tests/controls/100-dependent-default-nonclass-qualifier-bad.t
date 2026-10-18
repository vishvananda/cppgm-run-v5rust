// expect reject
template<class T, class U, class V = typename U::type> struct Arguments;
template<class T> struct Use {
  Arguments<T, int>* pointer;
};
