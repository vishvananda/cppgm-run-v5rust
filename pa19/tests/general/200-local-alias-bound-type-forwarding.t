template<class T>
struct remove_ref {
  typedef T type;
};

template<class T>
struct remove_ref<T&> {
  typedef T type;
};

template<class U>
struct Outer {
  template<class T>
  static typename remove_ref<T>::type&& mv(T&& t) {
    using V = typename remove_ref<T>::type;
    return static_cast<V&&>(t);
  }
};

int main() {
  int value = 7;
  int* p = &value;
  int** pp = &p;
  return **Outer<int*>::mv(pp) - 7;
}
