struct S {
  struct {
    unsigned char a : 1;
    unsigned char b : 7;
  };
  struct padding0 {};
  [[no_unique_address]] padding0 pad;
  char data[23];
};

static_assert(__builtin_offsetof(S, pad) == 0, "");
static_assert(__builtin_offsetof(S, data) == 1, "");
static_assert(sizeof(S) == 24, "");

int main() {
  S s;
  s.data[0] = 'x';
  return sizeof(s);
}
