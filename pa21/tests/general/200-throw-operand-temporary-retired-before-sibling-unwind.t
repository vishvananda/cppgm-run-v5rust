int temporary_destructions;
int scope_destructions;

struct Text
{
  int value;

  Text(int input = 0) : value(input)
  {
  }

  Text(const Text & other) : value(other.value)
  {
  }

  ~Text()
  {
    ++temporary_destructions;
  }
};

Text source(3);

Text operator+(const char *, const Text & right)
{
  return Text(right.value + 1);
}

struct Error
{
  int value;

  Error(const Text & text) : value(text.value)
  {
  }

  Error(const Error & other) : value(other.value)
  {
  }

  ~Error()
  {
  }
};

struct Scope
{
  ~Scope()
  {
    ++scope_destructions;
  }
};

void raise_later()
{
  throw 7;
}

void exercise(bool take_throw)
{
  Scope scope;
  if (take_throw)
    throw Error("unused" + source);
  raise_later();
}

int main()
{
  try
  {
    exercise(false);
  }
  catch (int)
  {
  }
  return temporary_destructions != 0 || scope_destructions != 1;
}
