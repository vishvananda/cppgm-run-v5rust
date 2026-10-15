// expect exit 0
// A conversion function's reference result does not extend its receiver's lifetime.
int destroyed = 0;
struct target { int value; } object = {7};
struct source {
  source() {}
  ~source() { ++destroyed; }
  operator const target&() { return object; }
};
int test(bool pick) {
  int before = destroyed;
  const target& bound = (pick ? source() : source());
  if (destroyed != before + 1) return 1;
  if (&bound != &object || bound.value != 7) return 2;
  const target& direct = source();
  return destroyed == before + 2 && &direct == &object ? 0 : 3;
}
int main() { return test(true) + test(false); }
