// N3485 focus: 13.3.1.2 [over.match.oper] operator candidate set
struct Base
{
};

void operator==(const Base &, const Base &);

struct Token : Base
{
  friend bool operator==(const Token &, const Token &)
  {
    return true;
  }
};

int main()
{
  Token lhs;
  Token rhs;
  return lhs == rhs ? 0 : 1;
}
