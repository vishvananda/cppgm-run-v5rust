// expect reject
// An unrelated dependent argument does not hide an invalid default type.
template<class T, class U, class V = U*> struct Arguments;
template<class T> struct Use {
  Arguments<T, int&>* pointer;
};
