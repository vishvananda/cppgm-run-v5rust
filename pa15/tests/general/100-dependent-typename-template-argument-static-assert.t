template<class T> struct Traits { typedef int type; };
template<class T> struct Pred { static const bool value = true; };

template<class T>
int f() {
  static_assert(Pred<typename Traits<T>::type>::value, "x");
  return 0;
}

int main() {
  return f<int>();
}
