typedef int& IntRef;

template<class T>
using add_const_t = const T;

void sink(int&) {}

int global_value = 0;

add_const_t<IntRef> get() {
  return global_value;
}

int main() {
  sink(get());
  return 0;
}
