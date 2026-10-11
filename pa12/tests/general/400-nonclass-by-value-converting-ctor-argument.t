struct Text {
  const char *data;
  int marker;

  Text(const char *p) : data(p), marker(p[0] == 'x' ? 7 : 3) {}
};

int take(Text t)
{
  return t.marker == 7 ? 0 : 1;
}

int main()
{
  return take("x");
}
