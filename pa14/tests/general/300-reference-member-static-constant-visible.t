template<class T> struct Traits { static const int size = sizeof(T); };
template<class T> struct User { typedef int arr_type[Traits<T>::size]; };
User<int>::arr_type *f();
