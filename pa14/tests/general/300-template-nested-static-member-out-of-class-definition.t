namespace n {
template<class T>
struct outer {
  struct inner {
    static int value;
  };

  static int read() {
    return inner::value;
  }
};

template<class T>
int outer<T>::inner::value = 7;
}

int main() {
  return n::outer<int>::read() - 7;
}
