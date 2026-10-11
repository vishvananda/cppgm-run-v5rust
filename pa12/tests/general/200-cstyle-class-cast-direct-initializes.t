// VALIDATION: compile-pass

struct Token
{
  Token(unsigned int code = 0) : code_(code) {}

  unsigned int code_;
};

unsigned int convert(int value)
{
  Token ch = (Token)value;
  return ch.code_;
}

int main()
{
  return convert(7) == 7u ? 0 : 1;
}
