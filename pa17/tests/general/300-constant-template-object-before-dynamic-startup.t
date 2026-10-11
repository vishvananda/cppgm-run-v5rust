struct Text
{
  const char *value;
  constexpr Text(const char *text) : value(text) {}
};

template<class T> struct Storage
{
  static constexpr Text text = Text("one");
};
template<class T> constexpr Text Storage<T>::text;

int read();
int observed = read();

int read()
{
  return Storage<int>::text.value[0];
}

int main()
{
  return observed == 'o' && Storage<int>::text.value[2] == 'e' ? 0 : 1;
}
