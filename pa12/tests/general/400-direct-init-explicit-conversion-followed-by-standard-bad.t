struct X { explicit operator bool() const { return true; } };
int main() {
  X x;
  char c(x);
  return c;
}
