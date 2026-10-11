template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class T>
struct Box {
  typedef typename enable_if<true, T>::type type;
};

template<class T>
using X = typename Box<T>::type;

X<int> g();

int main() {
  return 0;
}
