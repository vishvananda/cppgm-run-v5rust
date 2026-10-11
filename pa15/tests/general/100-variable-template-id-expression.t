// HHC-185
template<class T>
constexpr unsigned long datasizeof_v = sizeof(T);

template<class T>
unsigned long f() {
  return datasizeof_v<T>;
}

int main() { return (int)(f<int>() - sizeof(int)); }
