// HHC-174
template<class T> struct remove_ref { typedef T type; };

template<class T>
typename remove_ref<T>::type* foo(typename remove_ref<T>::type* p) {
  return p;
}

unsigned long* h(unsigned long* p) {
  return foo<unsigned long>(p);
}
