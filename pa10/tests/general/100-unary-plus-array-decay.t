int main() {
  int a[2] = {3, 4};
  int *p = +a;
  return p[1] - 4;
}
