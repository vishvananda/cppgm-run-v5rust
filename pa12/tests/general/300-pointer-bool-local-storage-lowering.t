int main() {
  int x = 0;
  int *p = &x;
  bool b = p;
  return int(b) == 1 ? 0 : int(b);
}
