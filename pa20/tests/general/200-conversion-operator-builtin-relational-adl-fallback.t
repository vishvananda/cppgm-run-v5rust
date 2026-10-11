namespace n {
struct token {};

struct number {
  operator int() const { return 1; }
};

bool operator>(token, token);
}

int main()
{
  n::number x;
  return x > 0;
}
