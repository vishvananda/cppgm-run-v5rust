int pick(int &) { return 1; }
int pick(const int &) { return 0; }

constexpr int value = 1;

int main()
{
  return pick(value);
}
