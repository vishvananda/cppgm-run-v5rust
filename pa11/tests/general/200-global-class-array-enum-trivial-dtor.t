enum Code {
  first = 1,
  second = 2
};

struct Entry {
  int native;
  Code mapped;
};

static const Entry table[] = {
  {13, first},
  {30, second}
};

int main()
{
  return table[1].mapped == second ? 0 : 1;
}
