int alive;

struct Token
{
  Token() noexcept { ++alive; }
  Token(const Token&) noexcept { ++alive; }
  ~Token() noexcept { --alive; }
};

struct Empty
{
  Empty(Token, Token) noexcept {}
};

struct Holder
{
  Empty member;
  int tag;
};

int main()
{
  {
    Token first;
    Token second;
    Holder holder{Empty(first, second), 7};
    if (alive != 2 || holder.tag != 7) return 1;
  }
  return alive == 0 ? 0 : 2;
}
