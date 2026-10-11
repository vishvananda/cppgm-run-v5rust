struct Ptr
{
  int value;
};

bool operator!=(Ptr const & p, nullptr_t)
{
  return p.value != 0;
}

int main()
{
  Ptr p = {1};
  return p != 0 ? 0 : 1;
}
