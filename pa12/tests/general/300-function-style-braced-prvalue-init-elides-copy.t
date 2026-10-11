struct other {
  int high;
  int low;
};

struct value {
  int high;
  int low;

  value(const value& v) : high(v.high), low(v.low) {}
  value(int high_, int low_) : high(high_), low(low_) {}
  value(const other& v) : high(v.high), low(v.low) {}
};

int main() {
  value mask = value{1, 0};
  return mask.high - 1;
}
