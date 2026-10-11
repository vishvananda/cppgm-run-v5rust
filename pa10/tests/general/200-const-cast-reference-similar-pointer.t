int main()
{
  char text[2] = {'x', 0};
  char * value = text;
  const char *& view = const_cast<const char *&>(value);
  return view[0] == 'x' ? 0 : 1;
}
