int f(int x) {
  int y = 0;
  switch (x) {
    case 0:
      y = 1;
      break;
    case 1:
    case 2:
      y = 3;
      break;
    default:
      y = 4;
  }
  return y;
}

int main() {
  return f(2) - 3;
}
