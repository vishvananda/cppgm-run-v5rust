template <class T>
int f() {
  if ([[__maybe_unused__]] int n = sizeof(T))
    return n;
  return 0;
}

int main() {
  return f<int>();
}
