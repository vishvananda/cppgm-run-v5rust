template<class T>
void f() {
  using U = T;
  U x = 0;
  (void)x;
}

int main() {
  f<int>();
  return 0;
}
