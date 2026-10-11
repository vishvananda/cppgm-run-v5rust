namespace N {
typedef int count_t;

template<class T>
struct Holder {
  template<class... Args>
  int make(count_t h, Args&&... args);
  int run();
};

template<class T>
template<class... Args>
int Holder<T>::make(count_t h, Args&&... args) {
  return h + sizeof...(Args);
}

template<class T>
int Holder<T>::run() {
  return make(3, 1, 2);
}
}

int main() {
  N::Holder<int> h;
  return h.run() == 5 ? 0 : 1;
}
