template<class T> struct remove_ref { typedef T type; };
template<class T> struct remove_ref<T&> { typedef T type; };

template<class T>
typename remove_ref<T>::type&& mv(T&& t) {
  return static_cast<typename remove_ref<T>::type&&>(t);
}

const char16_t* f(const char16_t* p) {
  return mv(p);
}

int main() { return 0; }
