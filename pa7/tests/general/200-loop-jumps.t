int f() {
  int x = 2;
  while (x > 0) {
    x = x - 1;
    if (x == 1)
      break;
    continue;
  }
  return x;
}
