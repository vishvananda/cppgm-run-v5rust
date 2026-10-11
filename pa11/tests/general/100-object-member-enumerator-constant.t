struct Holder
{
  enum { max_buffers = 16 };
  int count;
};

int main()
{
  Holder result;
  result.count = 0;
  while(result.count < result.max_buffers) {
    ++result.count;
  }
  return result.count == 16 ? 0 : 1;
}
