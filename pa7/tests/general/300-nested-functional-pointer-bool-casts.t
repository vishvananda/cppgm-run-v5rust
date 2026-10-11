int main() {
  int x = 0;
  int *p = &x;
  return int(bool(p)) == 1 ? 0 : int(bool(p));
}
