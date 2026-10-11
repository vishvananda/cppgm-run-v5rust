namespace std
{
  struct nothrow_t {};
  extern const nothrow_t nothrow;
}

const std::nothrow_t std::nothrow = std::nothrow_t();

void *operator new(unsigned long, const std::nothrow_t&) noexcept
{
  return 0;
}

int constructed = 0;

struct value
{
  value()
  {
    constructed = 1;
  }
};

int main()
{
  value *p = new (std::nothrow) value();
  if (p)
    return 2;
  return constructed;
}
