// N3485 focus: 13.3.3.2 [over.ics.rank] derived-to-base conversion ranking
struct Root
{
  int tag;
};

struct Base : Root
{
};

struct Wrapper : Base
{
};

int select(Base &)
{
  return 1;
}

int select(Root &)
{
  return 2;
}

int main()
{
  Wrapper wrapper;
  return select(wrapper) == 1 ? 0 : 1;
}
