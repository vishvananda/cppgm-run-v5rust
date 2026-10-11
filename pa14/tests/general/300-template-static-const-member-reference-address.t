template<class T>
struct Holder {
  static const int value = 7;
};

template<class T>
const int Holder<T>::value;

int read_ref(const int& r) {
  return r;
}

int main() {
  const int& ref = Holder<int>::value;
  return read_ref(ref) - 7;
}
