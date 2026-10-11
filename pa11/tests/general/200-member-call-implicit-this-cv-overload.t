// VALIDATION: compile-pass
// N3485 focus: 13.3.1 [over.match.funcs], 13.3.3 [over.match.best]

struct Box {
  int value;

  int pick()
  {
    return value;
  }

  int pick() const
  {
    return 100;
  }

  int read()
  {
    return pick();
  }
};

int main()
{
  Box box = {7};
  return box.read() == 7 ? 0 : 1;
}
