int main() {
  int x = 3;
  int& y = x;
  y = 5;
  return x - 5;
}
