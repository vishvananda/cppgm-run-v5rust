// VALIDATION: a const reference bound to a scalar prvalue (a literal, a call
// result, an arithmetic result, a converting call, a conditional) gets
// storage of its own for the reference's lifetime.

int counter = 40;

int next()
{
  return ++counter;
}

struct holder
{
  int value;
};

holder make(int value)
{
  holder result = {value};
  return result;
}

int literal()
{
  const int& bound = 5;
  return bound;
}

int call()
{
  const int& bound = next();
  return bound + next() - 42;
}

long converting()
{
  const long& bound = next();
  return bound;
}

int arithmetic()
{
  const int& bound = 1 + next();
  return bound;
}

int conditional(int select)
{
  const int& bound = select ? make(7).value : 0;
  return bound;
}

const int& global_bound = next();

int main()
{
  volatile int clobber = 99;
  if (counter != 41 || global_bound != counter) return 8;
  if (literal() != 5) return 1;
  int before = counter;
  if (call() != (before + 1) + (before + 2) - 42 ||
    counter != before + 2) return 2;
  before = counter;
  if (converting() != before + 1 || counter != before + 1) return 3;
  before = counter;
  if (arithmetic() != before + 2 || counter != before + 1) return 4;
  if (conditional(1) != 7) return 5;
  if (conditional(0) != 0) return 6;
  if (global_bound != 41) return 7;
  if (clobber != 99) return 9;
  return 0;
}
