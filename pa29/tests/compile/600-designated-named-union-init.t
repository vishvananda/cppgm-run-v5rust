struct Token {
  int kind;
  union Value {
    unsigned long u;
    long s;
  } value;
};

int main() {
  Token token = {0, {.u = 7}};
  return token.kind == 0 && token.value.u == 7 ? 0 : 1;
}
