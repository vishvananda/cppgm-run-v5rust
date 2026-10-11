namespace lib
{

struct AtomicLike
{
  operator long() const
  {
    return 0;
  }
};

struct Executor
{
  Executor(const AtomicLike&)
  {
  }

  Executor(int)
  {
  }
};

bool operator==(const Executor&, const Executor&)
{
  return false;
}

int run()
{
  AtomicLike count;
  return count == 0 ? 0 : 1;
}

}

int main()
{
  return lib::run();
}
