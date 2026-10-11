template <class T>
void f(unsigned long n) {
  [[__maybe_unused__]] unsigned long bytes = n * sizeof(T);
}

int main() {
  f<int>(1);
  return 0;
}
