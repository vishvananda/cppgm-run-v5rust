int selected;

void operator delete(void *) noexcept
{
  selected = 1;
}

void operator delete(void *, void *) noexcept
{
  selected = 2;
}

int main()
{
  int *p = new int;
  delete p;
  return selected == 1 ? 0 : selected;
}
