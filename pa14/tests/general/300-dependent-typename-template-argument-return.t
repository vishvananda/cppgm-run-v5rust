template<class T> struct Traits { typedef int type; };
template<class T> struct Pred { static const bool value = true; };

template<class T>
bool f(T x) {
  return Pred<typename Traits<T>::type>::value;
}

int main() {
  return f(0) ? 0 : 1;
}
