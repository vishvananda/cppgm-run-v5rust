int f() {
  int x = 1;
  int y = 2;
  int z = (x++, y << 1) | (~x & 7);
  return z;
}
