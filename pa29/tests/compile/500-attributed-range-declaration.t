int f() {
  int xs[1] = {1};
  for ([[__maybe_unused__]] int x : xs)
    return x;
  return 0;
}

int main() {
  return f();
}
