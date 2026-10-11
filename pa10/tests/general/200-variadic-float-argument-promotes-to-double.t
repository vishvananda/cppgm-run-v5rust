extern "C" int sink(const char *, ...);

int main()
{
  float value = 1.25F;
  return sink("", value);
}
