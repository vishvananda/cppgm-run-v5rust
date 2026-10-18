// expect reject
template<class T, class U, class V = U()> struct Arguments;
template<class T> struct Use {
  Arguments<T, int[2]>* pointer;
};
