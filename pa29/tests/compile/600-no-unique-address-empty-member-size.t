struct padding0 {};

struct Short {
  unsigned char size;
  [[no_unique_address]] padding0 pad;
  char data[23];
};

static_assert(sizeof(Short) == 24, "");

int main() {
  Short s;
  s.size = 1;
  s.data[0] = 'x';
  return sizeof(s);
}
