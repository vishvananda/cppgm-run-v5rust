int f() {
  int a[3];
  a[1] = 5;
  return a[1] + sizeof(a);
}

int main() {
  return f();
}
