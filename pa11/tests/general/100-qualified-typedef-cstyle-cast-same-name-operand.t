struct owner
{
  typedef unsigned int mask;
};

int run(unsigned int mask)
{
  return (owner::mask)mask;
}

int main()
{
  return run(0);
}
