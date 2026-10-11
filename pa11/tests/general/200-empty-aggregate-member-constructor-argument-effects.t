int calls;
int next() noexcept { return ++calls; }

struct Empty
{
  Empty(int, int) noexcept {}
};

struct Holder
{
  Empty member;
  int tag;
};

int main()
{
  Holder holder{Empty{next(), next()}, 7};
  return calls == 2 && holder.tag == 7 ? 0 : 1;
}
