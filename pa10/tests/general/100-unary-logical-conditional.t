int f() {
  int x = 0;
  int y = 1;
  bool b = !x || (y && true);
  ++x;
  y++;
  return b ? x : y;
}

int main() {
  return f();
}
