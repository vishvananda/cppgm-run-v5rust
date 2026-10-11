template<class T>
int add_impl(int& st) {
  st += sizeof(T);
  return st;
}

template<class... T>
int add_each() {
  int st = 1;
  typedef int A[sizeof...(T) + 1];
  (void)A{0, add_impl<T>(st)...};
  return st;
}

int main() {
  return add_each<>() == 1 && add_each<int, char>() == 6 ? 0 : 1;
}
