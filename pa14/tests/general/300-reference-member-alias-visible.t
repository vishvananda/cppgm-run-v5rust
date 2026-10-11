template<class T> struct Alloc { typedef T value_type; };
template<class T> struct Vec { typedef typename Alloc<T>::value_type elem_type; };
Vec<int>::elem_type *f();
