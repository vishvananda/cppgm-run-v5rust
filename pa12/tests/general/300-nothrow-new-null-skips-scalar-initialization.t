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

int main()
{
  int *pointer = new (std::nothrow) int(7);
  return pointer == 0 ? 0 : 1;
}
