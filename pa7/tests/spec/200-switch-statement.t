// N3485 focus: 6.4.2 [stmt.switch] switch statement
int f(int x) {
  switch (x) {
    case 0:
      return 1;
    case 1:
    case 2:
      return 3;
    default:
      return 4;
  }
}

int main() {
  return f(2) - 3;
}
