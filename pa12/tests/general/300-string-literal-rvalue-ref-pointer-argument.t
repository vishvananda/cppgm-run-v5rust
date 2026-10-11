const char *saved = 0;

void sink(const char *&&p)
{
  saved = p;
}

int main()
{
  sink("Bar");
  return saved ? 0 : 1;
}
