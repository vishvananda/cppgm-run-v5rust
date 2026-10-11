template<class T> struct Holder { static T bad() { return T::missing_name; } typedef T type; };
template<class T> using held_type = typename Holder<T>::type;
held_type<int> *f();
