template<class T> struct Traits { typedef int type; };
template<class T> struct Pred { static const bool value = true; };

template<class T>
bool f(T x) {
  bool b = Pred<typename Traits<T>::type>::value;
  return b;
}

int main() {
  return f(0) ? 0 : 1;
}
