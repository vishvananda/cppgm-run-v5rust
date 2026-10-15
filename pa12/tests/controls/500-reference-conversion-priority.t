// expect exit 0
// Direct reference-result binding wins before temporary construction (8.5.3).
int constructors = 0;
int conversions = 0;
struct source;
struct target {
  int value;
  target(int n): value(n) {}
  target(source&): value(0) { ++constructors; }
};
struct source {
  target* object;
  operator const target&() const { ++conversions; return *object; }
};
int read(const target& value) { return value.value; }
int main() {
  target object(7);
  source value = {&object};
  const target& bound = ((value));
  int result = read(value);
  return &bound == &object && result == 7 && constructors == 0 && conversions == 2 ? 0 : 1;
}
