struct Token {
  int kind;
  union Value {
    unsigned long u;
    long s;
  } value;
};

unsigned long read_value(unsigned long value) {
  return ((Token){0, {.u = value}}).value.u;
}

int main() {
  return read_value(9) == 9 ? 0 : 1;
}
