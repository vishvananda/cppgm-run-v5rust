// HHC-060
template<class T> struct remove_cv { typedef T type; };
template<class T, bool = true> struct make_unsigned { typedef int type; };

template<class T>
struct X {
  typedef typename make_unsigned<typename remove_cv<T>::type>::type type;
};

int main() { return 0; }
