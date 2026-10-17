// N3485 3.3.10p1, 14.5.3p5: local shadows expire at block and loop boundaries;
// expect exit 0
int sum(int a, int b) { return a + b; }
template<class... T> int block(T... t)
{
  int r = 0;
  { int t = 10; int a[] = { (t + (int)sizeof(T))... }; r = a[0] + a[1]; }
  return r + sum(t...);
}
template<class... T> int loop(T... t)
{
  int r = 0;
  for (int t = 1; t < 2; ++t)
  { int a[] = { (t + (int)sizeof(T))... }; r = a[0] + a[1]; }
  return r + sum(t...);
}
int main() { return block(1, 2) != 31 || loop(1, 2) != 13; }
