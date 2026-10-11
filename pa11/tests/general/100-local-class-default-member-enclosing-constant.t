int shared = 5;

int main()
{
  const unsigned sentinel = 37;
  int automatic = 9;
  struct Record
  {
    unsigned value = sentinel;
    unsigned read(int argument)
    {
      int local = 2;
      return sentinel + shared + argument + local + sizeof(automatic);
    }
  };
  Record record;
  return record.value == 37 && record.read(3) == 51 ? 0 : 1;
}
