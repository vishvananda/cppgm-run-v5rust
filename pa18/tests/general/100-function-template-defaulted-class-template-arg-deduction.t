template<class T, class U = int>
struct Box {
  T value;
};

template<class T>
T unwrap(Box<T> arg) {
  return arg.value;
}

int main() {
  Box<long> arg = {7};
  return unwrap(arg) == 7 ? 0 : 1;
}
