int f(int x) {
  switch (x) {
    case 0:
      return 0;
    default:
      int y = 4;
      return y;
  }
}

int main() {
  return f(1) - 4;
}
