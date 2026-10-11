int live;

struct record
{
  record() { ++live; }
  record(const record &) { ++live; }
  ~record() { --live; }
};

record make()
{
  throw 1;
}

void exercise()
{
  record enclosing;
  record destination = true ? make() : record();
  (void)destination;
}

int main()
{
  try {
    exercise();
  } catch(int) {
    return live == 0 ? 0 : 1;
  }
  return 2;
}
