int main() {
  int a[3] = {1, 2, 3};
  int s = 0;
  for (const int& x : a)
    s = s + x;
  return s;
}
