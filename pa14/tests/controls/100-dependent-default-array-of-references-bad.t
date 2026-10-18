// expect reject
template<class T, class U, class V = U[2]> struct Arguments;
template<class T> struct Use {
  Arguments<T, int&>* pointer;
};
