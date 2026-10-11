// N3485 focus: 5.7 [expr.add], 12.3.2 [class.conv.fct]
// Built-in pointer arithmetic can use a class operand converted to integral.
struct offset {
  offset(int value) : value_(value) {}

  operator int() const {
    return value_;
  }

private:
  int value_;
};

int read_at(int *first, offset n) {
  return *(first + n);
}

int read_reverse(int *first, offset n) {
  return *(n + first);
}

int read_before(int *last, offset n) {
  return *(last - n);
}

int main() {
  int values[4] = { 2, 3, 5, 7 };
  offset one(1);
  offset two(2);
  return read_at(values, two) == 5 &&
         read_reverse(values, one) == 3 &&
         read_before(values + 3, two) == 3 ? 0 : 1;
}
