struct string_like {
  int value;
  string_like() : value(3) {}
};

struct path {
  template<class String>
  String string() const;
};

template<>
inline string_like path::string<string_like>() const
{
  return string_like();
}

int main()
{
  path p;
  return p.string<string_like>().value == 3 ? 0 : 1;
}
