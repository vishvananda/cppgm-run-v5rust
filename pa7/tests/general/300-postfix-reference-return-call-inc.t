int & f(int & x) { return x; }
int main() {
  int x = 0;
  f(x)++;
  return x;
}
