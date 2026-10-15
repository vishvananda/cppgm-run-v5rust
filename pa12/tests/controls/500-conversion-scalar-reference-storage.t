// expect exit 0
// A scalar conversion result owns reference binding storage in its full expression.
int conversions = 0;
struct source {
  operator short() const { ++conversions; return 7; }
};
int read(const long& value) { return value == 7 ? 0 : 1; }
int main() {
  source value;
  const int& bound = value;
  int result = read(value);
  return bound == 7 && result == 0 && conversions == 2 ? 0 : 1;
}
