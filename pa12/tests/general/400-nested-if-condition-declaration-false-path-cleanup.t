// A class-valued condition in a braceless nested if exists only when control
// reaches its declaration; the enclosing false path has nothing to destroy.

int destroyed;

struct probe
{
  bool value;

  explicit probe(bool input) : value(input) {}
  ~probe() { ++destroyed; }
  operator bool() const { return value; }
};

void run(int kind)
{
  if (kind == 1)
    if (probe condition = probe(true))
      return;
}

int main()
{
  run(0);
  if (destroyed != 0)
    return 1;
  run(1);
  return destroyed == 1 ? 0 : 2;
}
