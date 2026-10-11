template<class T>
struct C {
  C(C&&);
};

template<class T>
inline __attribute__((visibility("hidden"))) C<T>::C(C&&) {}
