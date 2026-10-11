enum Mode {
  ModeValue __attribute((__unused__)) = 3
};

void consume(int* __attribute((__unused__)) p);
void consume(int*) {}

int main() {
  int value = ModeValue;
  consume(&value);
  return value - 3;
}
