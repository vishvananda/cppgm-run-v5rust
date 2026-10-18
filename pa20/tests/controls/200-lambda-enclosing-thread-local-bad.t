// expect reject
int main()
{
  thread_local int value = 2;
  auto read = [&] { return ++value; };
  return read();
}
