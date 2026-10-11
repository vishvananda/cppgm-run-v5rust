template<class CharT>
int fill(CharT *p, CharT value)
{
  p[0] = value;
  return 0;
}

struct Buffer
{
  char data[2];
  char other[2];

  Buffer()
  {
    data[0] = 0;
    data[1] = 0;
    other[0] = 'z';
    other[1] = 0;
  }

  const char &operator[](int i) const
  {
    return other[i];
  }

  char &operator[](int i)
  {
    return data[i];
  }
};

int main()
{
  Buffer buffer;
  fill(&buffer[0], 'a');
  return buffer.data[0] == 'a' && buffer.other[0] == 'z' ? 0 : 1;
}
