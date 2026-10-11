int f() {
  for ([[__maybe_unused__]] int i = 0; i < 1; ++i)
    return i;
  return 0;
}

int main() {
  return f();
}
